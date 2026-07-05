/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 */

use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};
use serde::{Deserialize, Serialize};

/* ── Color Palette ───────────────────────────────────────────────────────── */

const C_TRANSPARENT: u32 = 0x00000000;
const C_BG_GLASS_PILL: u32 = 0xCC10101C;
const C_BG_GLASS_CHAT: u32 = 0xF5131322;
const C_BORDER_GLOW: u32 = 0x887C5CBF;  /* Purple */
const C_BORDER_DIM: u32 = 0x33EEEEF4;
const C_ACCENT: u32 = 0xFFE03E8A;      /* Pink */
const C_TEXT: u32 = 0xFFEEEEF4;
const C_TEXT_DIM: u32 = 0xFF9898AA;
const C_TEXT_MUTED: u32 = 0xFF454560;
const C_CURSOR: u32 = 0xFF3ABFAA;      /* Teal */

#[derive(Serialize)]
struct CommandReq {
    cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum DaemonResp {
    #[serde(rename = "mode")]
    Mode { mode: String },
    #[serde(rename = "chunk")]
    Chunk { text: String },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { text: String },
}

struct IslandApp {
    mode: String,
    expanded: bool,
    input_text: String,
    ai_text: String,
    cursor_tick: u32,
    waiting: bool,
    stream: Option<UnixStream>,
    rx_buf: Vec<u8>,
}

impl IslandApp {
    fn new() -> Self {
        let stream = UnixStream::connect("/run/luna-ai.sock").ok();
        if let Some(ref s) = stream {
            let _ = s.set_nonblocking(true);
        }
        
        let mut app = Self {
            mode: String::from("AMBIENT"),
            expanded: false,
            input_text: String::new(),
            ai_text: String::from("Hi, I'm Mahina. How can I help you today?"),
            cursor_tick: 0,
            waiting: false,
            stream,
            rx_buf: Vec::new(),
        };

        // Query initial mode
        app.send_cmd("get_mode", None);
        app
    }

    fn send_cmd(&mut self, cmd: &str, text: Option<&str>) {
        if let Some(ref mut s) = self.stream {
            let req = CommandReq {
                cmd: cmd.to_string(),
                text: text.map(|t| t.to_string()),
                app: None,
            };
            if let Ok(json) = serde_json::to_string(&req) {
                let _ = s.write_all(json.as_bytes());
                let _ = s.write_all(b"\n");
            }
        }
    }

    fn update_socket(&mut self) {
        if self.stream.is_none() {
            // Attempt reconnect
            if let Ok(s) = UnixStream::connect("/run/luna-ai.sock") {
                let _ = s.set_nonblocking(true);
                self.stream = Some(s);
                self.send_cmd("get_mode", None);
            }
            return;
        }

        let mut buf = [0u8; 1024];
        let stream = self.stream.as_mut().unwrap();
        match stream.read(&mut buf) {
            Ok(0) => {
                self.stream = None;
            }
            Ok(n) => {
                self.rx_buf.extend_from_slice(&buf[..n]);
                self.process_lines();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                self.stream = None;
            }
        }
    }

    fn process_lines(&mut self) {
        while let Some(pos) = self.rx_buf.iter().position(|&b| b == b'\n') {
            let line_bytes = self.rx_buf.drain(..pos + 1).collect::<Vec<u8>>();
            if let Ok(line_str) = std::str::from_utf8(&line_bytes[..line_bytes.len() - 1]) {
                if let Ok(resp) = serde_json::from_str::<DaemonResp>(line_str) {
                    match resp {
                        DaemonResp::Mode { mode } => {
                            self.mode = mode;
                        }
                        DaemonResp::Chunk { text } => {
                            if self.waiting {
                                self.ai_text.clear();
                                self.waiting = false;
                            }
                            self.ai_text.push_str(&text);
                        }
                        DaemonResp::Done => {
                            self.waiting = false;
                        }
                        DaemonResp::Error { text } => {
                            self.ai_text = format!("Error: {}", text);
                            self.waiting = false;
                        }
                    }
                }
            }
        }
    }

    fn render(&mut self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        self.cursor_tick += 1;

        // Clear surface to fully transparent
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, C_TRANSPARENT);

        if !self.expanded {
            // Pill mode: 200 x 36, top-centered on a 360 x 240 canvas
            let pill_w = 200;
            let pill_h = 36;
            let px = (width as i32 - pill_w) / 2;
            let py = 10;

            // Blur glass shadow/bg
            fill_rect_on_slice(pixels, stride, width, height, px, py, pill_w, pill_h, C_BG_GLASS_PILL);
            
            // Subtle glowing border
            self.draw_outline(pixels, stride, width, height, px, py, pill_w, pill_h, C_BORDER_GLOW);

            // Left state dot (animated pulse)
            let dot_color = match self.mode.as_str() {
                "DEVSHELL" => C_ACCENT,
                "FOCUS" => 0xFF3ABFAA, /* Teal */
                "STUDY" => 0xFF7C5CBF, /* Purple */
                "CREATIVE" => 0xFFFFD166, /* Gold */
                _ => 0xFFEEEEF4, /* White */
            };

            let show_dot = (self.cursor_tick / 5) % 2 == 0;
            if show_dot {
                fill_rect_on_slice(pixels, stride, width, height, px + 12, py + 14, 8, 8, dot_color);
            }

            // Label
            let label = format!("MAHINA: {}", self.mode);
            draw_text_on_slice(pixels, stride, width, px + 28, py + 10, &label, C_TEXT);
        } else {
            // Chat overlay: 340 x 220
            let cx = 10;
            let cy = 10;
            let cw = 340;
            let ch = 220;

            fill_rect_on_slice(pixels, stride, width, height, cx, cy, cw, ch, C_BG_GLASS_CHAT);
            self.draw_outline(pixels, stride, width, height, cx, cy, cw, ch, C_BORDER_GLOW);

            // Header
            draw_text_on_slice(pixels, stride, width, cx + 12, cy + 12, "MAHINA AI ASSISTANT", C_TEXT_DIM);
            fill_rect_on_slice(pixels, stride, width, height, cx + 12, cy + 30, cw - 24, 1, C_BORDER_DIM);

            // Response area
            // Wrap text in a basic way
            let lines = self.wrap_text(&self.ai_text, 36);
            let mut ly = cy + 38;
            for line in lines.iter().take(6) {
                draw_text_on_slice(pixels, stride, width, cx + 12, ly, line, C_TEXT);
                ly += 18;
            }

            if self.waiting {
                let wait_dots = match (self.cursor_tick / 3) % 4 {
                    0 => ".  ",
                    1 => ".. ",
                    2 => "...",
                    _ => "   ",
                };
                draw_text_on_slice(pixels, stride, width, cx + 12, ly, &format!("Thinking{}", wait_dots), C_ACCENT);
            }

            // Input field divider
            fill_rect_on_slice(pixels, stride, width, height, cx + 12, cy + ch - 40, cw - 24, 1, C_BORDER_DIM);

            // Input prompt
            draw_text_on_slice(pixels, stride, width, cx + 12, cy + ch - 28, ">", C_TEXT_DIM);

            // Input text
            let disp_text = if self.input_text.is_empty() {
                "Ask a question..."
            } else {
                &self.input_text
            };
            let text_color = if self.input_text.is_empty() { C_TEXT_MUTED } else { C_TEXT };
            draw_text_on_slice(pixels, stride, width, cx + 28, cy + ch - 28, disp_text, text_color);

            // Blinking cursor
            if !self.input_text.is_empty() && (self.cursor_tick / 5) % 2 == 0 {
                let cur_x = cx + 28 + (self.input_text.len() as i32 * 8);
                fill_rect_on_slice(pixels, stride, width, height, cur_x, cy + ch - 28, 2, 14, C_CURSOR);
            }

            // Close prompt at bottom right
            draw_text_on_slice(pixels, stride, width, cx + cw - 90, cy + 12, "[Esc] Close", C_TEXT_MUTED);
        }
    }

    fn draw_outline(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32,
                    x: i32, y: i32, w: i32, h: i32, col: u32) {
        fill_rect_on_slice(pixels, stride, width, height, x, y, w, 1, col);
        fill_rect_on_slice(pixels, stride, width, height, x, y + h - 1, w, 1, col);
        fill_rect_on_slice(pixels, stride, width, height, x, y, 1, h, col);
        fill_rect_on_slice(pixels, stride, width, height, x + w - 1, y, 1, h, col);
    }

    fn wrap_text(&self, text: &str, limit: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > limit {
                lines.push(current_line.clone());
                current_line = word.to_string();
            } else {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    fn handle_click(&mut self, px: i32, py: i32) {
        if !self.expanded {
            // Click inside the 200 x 36 pill?
            let pill_w = 200;
            let pill_h = 36;
            let rx = (360 - pill_w) / 2;
            let ry = 10;
            if px >= rx && px < rx + pill_w && py >= ry && py < ry + pill_h {
                self.expanded = true;
            }
        } else {
            // Click outside the 340 x 220 chat?
            let cx = 10;
            let cy = 10;
            let cw = 340;
            let ch = 220;
            if px < cx || px >= cx + cw || py < cy || py >= cy + ch {
                self.expanded = false;
            }
        }
    }

    fn handle_char(&mut self, ch: char) {
        if self.expanded {
            if ch == '\n' {
                if !self.input_text.is_empty() {
                    let prompt = self.input_text.clone();
                    self.input_text.clear();
                    self.ai_text = String::from("Contacting Ollama...");
                    self.waiting = true;
                    self.send_cmd("prompt", Some(&prompt));
                }
            } else if ch == '\x7f' {
                self.input_text.pop();
            } else if ch >= ' ' && ch <= '~' {
                if self.input_text.len() < 32 {
                    self.input_text.push(ch);
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER)?;
    
    // The surface covers 360 x 240 (at top center of the 1024x768 screen)
    let width = 360;
    let height = 240;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 332, 10,  /* x=332 (centered on 1024), y=10 */
        width, height, LGP_LAYER_LUNA_ISLAND, true,
    )?;

    conn.set_nonblocking(true)?;

    let mut app = IslandApp::new();

    loop {
        app.update_socket();

        {
            let pixels = surface.pixels();
            app.render(pixels, width * 4, width, height);
        }
        surface.commit(&mut conn)?;

        let mut pfd = pollfd {
            fd: conn.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };
        let ret = unsafe { poll(&mut pfd, 1, 100) };
        if ret < 0 { continue; }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.pressed {
                                if ev.key == 0x70029 { // Escape
                                    app.expanded = false;
                                } else if ev.key == 0x7002a { // Backspace
                                    app.handle_char('\x7f');
                                } else if ev.key == 0x70028 { // Enter
                                    app.handle_char('\n');
                                } else {
                                    // Basic mapping for alphanumeric
                                    if let Some(ch) = key_to_char(ev.key, ev.modifiers) {
                                        app.handle_char(ch);
                                    }
                                }
                            }
                        }
                    }
                    t if t == LgpMessageType::PointerButton as u16 => {
                        if let Some(ev) = parse_pointer_button(&msg.payload) {
                            if ev.pressed {
                                app.handle_click(ev.x, ev.y);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn key_to_char(key: u32, modifiers: u32) -> Option<char> {
    let shift = (modifiers & 1) != 0;
    // Map standard raw scancodes (scancode mapping matches keyboard.c)
    match key {
        0x70004 => Some(if shift { 'A' } else { 'a' }),
        0x70005 => Some(if shift { 'B' } else { 'b' }),
        0x70006 => Some(if shift { 'C' } else { 'c' }),
        0x70007 => Some(if shift { 'D' } else { 'd' }),
        0x70008 => Some(if shift { 'E' } else { 'e' }),
        0x70009 => Some(if shift { 'F' } else { 'f' }),
        0x7000a => Some(if shift { 'G' } else { 'g' }),
        0x7000b => Some(if shift { 'H' } else { 'h' }),
        0x7000c => Some(if shift { 'I' } else { 'i' }),
        0x7000d => Some(if shift { 'J' } else { 'j' }),
        0x7000e => Some(if shift { 'K' } else { 'k' }),
        0x7000f => Some(if shift { 'L' } else { 'l' }),
        0x70010 => Some(if shift { 'M' } else { 'm' }),
        0x70011 => Some(if shift { 'N' } else { 'n' }),
        0x70012 => Some(if shift { 'O' } else { 'o' }),
        0x70013 => Some(if shift { 'P' } else { 'p' }),
        0x70014 => Some(if shift { 'Q' } else { 'q' }),
        0x70015 => Some(if shift { 'R' } else { 'r' }),
        0x70016 => Some(if shift { 'S' } else { 's' }),
        0x70017 => Some(if shift { 'T' } else { 't' }),
        0x70018 => Some(if shift { 'U' } else { 'u' }),
        0x70019 => Some(if shift { 'V' } else { 'v' }),
        0x7001a => Some(if shift { 'W' } else { 'w' }),
        0x7001b => Some(if shift { 'X' } else { 'x' }),
        0x7001c => Some(if shift { 'Y' } else { 'y' }),
        0x7001d => Some(if shift { 'Z' } else { 'z' }),
        
        0x7001e => Some(if shift { '!' } else { '1' }),
        0x7001f => Some(if shift { '@' } else { '2' }),
        0x70020 => Some(if shift { '#' } else { '3' }),
        0x70021 => Some(if shift { '$' } else { '4' }),
        0x70022 => Some(if shift { '%' } else { '5' }),
        0x70023 => Some(if shift { '^' } else { '6' }),
        0x70024 => Some(if shift { '&' } else { '7' }),
        0x70025 => Some(if shift { '*' } else { '8' }),
        0x70026 => Some(if shift { '(' } else { '9' }),
        0x70027 => Some(if shift { ')' } else { '0' }),
        
        0x7002c => Some(' '),
        0x7002d => Some(if shift { '_' } else { '-' }),
        0x7002e => Some(if shift { '+' } else { '=' }),
        0x7002f => Some(if shift { '{' } else { '[' }),
        0x70030 => Some(if shift { '}' } else { ']' }),
        0x70031 => Some(if shift { '|' } else { '\\' }),
        0x70033 => Some(if shift { ':' } else { ';' }),
        0x70034 => Some(if shift { '"' } else { '\'' }),
        0x70035 => Some(if shift { '~' } else { '`' }),
        0x70036 => Some(if shift { '<' } else { ',' }),
        0x70037 => Some(if shift { '>' } else { '.' }),
        0x70038 => Some(if shift { '?' } else { '/' }),
        _ => None,
    }
}
