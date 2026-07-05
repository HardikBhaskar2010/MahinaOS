/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#include "inference.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <errno.h>

#define OLLAMA_HOST "127.0.0.1"
#define OLLAMA_PORT 11434

static void escape_json_string(const char *src, char *dst, size_t dst_size) {
    size_t d = 0;
    for (size_t s = 0; src[s] != '\0' && d + 4 < dst_size; s++) {
        char c = src[s];
        if (c == '"') {
            dst[d++] = '\\';
            dst[d++] = '"';
        } else if (c == '\\') {
            dst[d++] = '\\';
            dst[d++] = '\\';
        } else if (c == '\n') {
            dst[d++] = '\\';
            dst[d++] = 'n';
        } else if (c == '\r') {
            dst[d++] = '\\';
            dst[d++] = 'r';
        } else if (c == '\t') {
            dst[d++] = '\\';
            dst[d++] = 't';
        } else {
            dst[d++] = c;
        }
    }
    dst[d] = '\0';
}

/* 
 * Simple JSON field extractor.
 * Looks for "key":"value" or "key":value.
 */
static int extract_json_str_field(const char *json, const char *key, char *out, size_t out_size) {
    char key_pattern[64];
    snprintf(key_pattern, sizeof(key_pattern), "\"%s\":", key);
    
    const char *pos = strstr(json, key_pattern);
    if (!pos) return -1;
    
    pos += strlen(key_pattern);
    /* skip whitespace */
    while (*pos == ' ' || *pos == '\t') pos++;
    
    if (*pos == '"') {
        pos++; /* skip open quote */
        size_t idx = 0;
        while (*pos && *pos != '"' && idx + 1 < out_size) {
            if (*pos == '\\' && *(pos + 1) != '\0') {
                /* Simple escape handling */
                pos++;
                if (*pos == 'n') out[idx++] = '\n';
                else if (*pos == 't') out[idx++] = '\t';
                else out[idx++] = *pos;
            } else {
                out[idx++] = *pos;
            }
            pos++;
        }
        out[idx] = '\0';
        return 0;
    }
    return -1;
}

int inference_prompt(int client_fd, const char *prompt) {
    char escaped[4096];
    escape_json_string(prompt, escaped, sizeof(escaped));

    /* Check if the model is specified in environment or default to llama3 */
    const char *model = getenv("LUNA_AI_MODEL");
    if (!model) model = "llama3";

    char payload[8192];
    int payload_len = snprintf(payload, sizeof(payload),
        "{\"model\":\"%s\",\"prompt\":\"%s\",\"stream\":true}",
        model, escaped);

    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) {
        char err_msg[128];
        snprintf(err_msg, sizeof(err_msg), "{\"type\":\"error\",\"text\":\"socket() failed: %s\"}\n", strerror(errno));
        (void)write(client_fd, err_msg, strlen(err_msg));
        return -1;
    }

    /* 1-second timeout for connection */
    struct timeval tv = { .tv_sec = 2, .tv_usec = 0 };
    setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
    setsockopt(sock, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));

    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(OLLAMA_PORT);
    addr.sin_addr.s_addr = inet_addr(OLLAMA_HOST);

    if (connect(sock, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
        char err_msg[256];
        snprintf(err_msg, sizeof(err_msg),
            "{\"type\":\"error\",\"text\":\"Ollama connection failed. Is Ollama running on port 11434?\"}\n");
        (void)write(client_fd, err_msg, strlen(err_msg));
        close(sock);
        return -1;
    }

    char req[16384];
    int req_len = snprintf(req, sizeof(req),
        "POST /api/generate HTTP/1.1\r\n"
        "Host: localhost:11434\r\n"
        "Content-Type: application/json\r\n"
        "Content-Length: %d\r\n"
        "Connection: close\r\n\r\n"
        "%s",
        payload_len, payload);

    if (write(sock, req, (size_t)req_len) < 0) {
        char err_msg[128];
        snprintf(err_msg, sizeof(err_msg), "{\"type\":\"error\",\"text\":\"HTTP write failed\"}\n");
        (void)write(client_fd, err_msg, strlen(err_msg));
        close(sock);
        return -1;
    }

    /* Read response stream */
    char buf[8192];
    size_t buf_len = 0;
    ssize_t n;
    bool headers_passed = false;

    while ((n = read(sock, buf + buf_len, sizeof(buf) - buf_len - 1)) > 0) {
        buf_len += (size_t)n;
        buf[buf_len] = '\0';

        char *stream_start = buf;
        if (!headers_passed) {
            /* Find end of HTTP headers */
            char *hdr_end = strstr(buf, "\r\n\r\n");
            if (hdr_end) {
                headers_passed = true;
                stream_start = hdr_end + 4;
            } else {
                /* Still reading headers */
                if (buf_len >= sizeof(buf) - 1) {
                    /* Header too long */
                    buf_len = 0;
                }
                continue;
            }
        }

        /* Parse line by line */
        char *line = stream_start;
        char *next_line;
        while ((next_line = strchr(line, '\n')) != NULL) {
            *next_line = '\0';
            
            /* Process line */
            if (line[0] == '{') {
                char response_text[2048];
                if (extract_json_str_field(line, "response", response_text, sizeof(response_text)) == 0) {
                    char client_msg[4096];
                    char escaped_resp[4096];
                    escape_json_string(response_text, escaped_resp, sizeof(escaped_resp));
                    int len = snprintf(client_msg, sizeof(client_msg),
                        "{\"type\":\"chunk\",\"text\":\"%s\"}\n", escaped_resp);
                    (void)write(client_fd, client_msg, (size_t)len);
                }
                
                /* Check if done */
                if (strstr(line, "\"done\":true")) {
                    (void)write(client_fd, "{\"type\":\"done\"}\n", 16);
                }
            }

            line = next_line + 1;
        }

        /* Move remaining data to front of buffer */
        size_t processed = (size_t)(line - buf);
        if (processed < buf_len) {
            memmove(buf, line, buf_len - processed);
            buf_len -= processed;
        } else {
            buf_len = 0;
        }
    }

    close(sock);
    return 0;
}
