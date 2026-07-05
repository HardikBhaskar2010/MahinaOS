/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

#ifndef LUNA_AI_INFERENCE_H
#define LUNA_AI_INFERENCE_H

/*
 * inference_prompt() — Connects to Ollama on localhost:11434 and streams
 * response chunks back to client_fd in NDC-JSON format.
 * Returns 0 on success, -1 on error.
 */
int inference_prompt(int client_fd, const char *prompt);

#endif /* LUNA_AI_INFERENCE_H */
