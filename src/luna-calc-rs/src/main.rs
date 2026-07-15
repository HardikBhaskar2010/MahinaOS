use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};

const COLOR_BG: u32 = 0xFF1E1E28;
const COLOR_TEXT: u32 = 0xFFEEEEF4;
const COLOR_TEXT_SEC: u32 = 0xFF9898AA;
const COLOR_ACCENT: u32 = 0xFF6B7FD4;
const COLOR_RAISED: u32 = 0xFF262636;
const COLOR_HOVER: u32 = 0xFF2E2E42;
const COLOR_PRESSED: u32 = 0xFF181824;

struct CalcApp {
    display: String,
    expr: String,
    result: f64,
    has_result: bool,
    error: bool,
}

impl CalcApp {
    fn new() -> Self {
        Self {
            display: String::from("0"),
            expr: String::new(),
            result: 0.0,
            has_result: false,
            error: false,
        }
    }

    fn press(&mut self, ch: char) {
        if self.error {
            self.display = String::from("0");
            self.expr = String::new();
            self.error = false;
            self.has_result = false;
        }
        if self.has_result && ch.is_ascii_digit() {
            self.display = String::from("0");
            self.expr = String::new();
            self.has_result = false;
        }
        match ch {
            '0'..='9' => {
                if self.display == "0" {
                    self.display = ch.to_string();
                } else {
                    self.display.push(ch);
                }
                self.expr.push(ch);
            }
            '.' => {
                if !self.display.contains('.') {
                    self.display.push('.');
                    self.expr.push('.');
                }
            }
            '+' | '-' | '*' | '/' => {
                if self.has_result {
                    self.expr = self.display.clone();
                    self.has_result = false;
                }
                self.expr.push(ch);
                self.display = self.expr.clone();
            }
            '=' => {
                self.calculate();
            }
            'C' => {
                self.display = String::from("0");
                self.expr = String::new();
                self.error = false;
                self.has_result = false;
            }
            '\x08' => { // Backspace
                if self.display.len() > 1 {
                    self.display.pop();
                    self.expr.pop();
                } else {
                    self.display = String::from("0");
                    self.expr = String::new();
                }
            }
            _ => {}
        }
    }

    fn calculate(&mut self) {
        self.error = false;
        let expr = self.expr.replace('x', "*");
        let result = self.eval_simple(&expr);
        match result {
            Some(val) if val.is_finite() => {
                self.display = format!("{}", val);
                self.expr = self.display.clone();
                self.result = val;
                self.has_result = true;
            }
            _ => {
                self.display = String::from("Error");
                self.error = true;
            }
        }
    }

    fn eval_simple(&self, expr: &str) -> Option<f64> {
        let expr = expr.trim();
        if expr.is_empty() { return None; }

        let mut tokens = Vec::new();
        let mut ops = Vec::new();

        let chars: Vec<char> = expr.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Check if this is a sign (+ or -) rather than a binary operator.
            // A sign is at index 0, or is preceded by another operator.
            if (chars[i] == '-' || chars[i] == '+') && 
               (i == 0 || "+-*/".contains(chars[i - 1])) {
                // Parse a signed number
                let mut num_str = String::new();
                num_str.push(chars[i]);
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    num_str.push(chars[i]);
                    i += 1;
                }
                let val: f64 = num_str.parse().ok()?;
                tokens.push(val);
            } else if "+-*/".contains(chars[i]) {
                ops.push(chars[i]);
                i += 1;
            } else if chars[i].is_ascii_digit() || chars[i] == '.' {
                let mut num_str = String::new();
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    num_str.push(chars[i]);
                    i += 1;
                }
                let val: f64 = num_str.parse().ok()?;
                tokens.push(val);
            } else {
                return None;
            }
        }

        if tokens.len() != ops.len() + 1 { return None; }
        let mut val = tokens[0];
        for (idx, &op) in ops.iter().enumerate() {
            let next = tokens[idx + 1];
            match op {
                '+' => val += next,
                '-' => val -= next,
                '*' => val *= next,
                '/' => {
                    if next == 0.0 { return None; }
                    val /= next;
                }
                _ => return None,
            }
        }
        Some(val)
    }

    fn render(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, COLOR_BG);
        fill_rect_on_slice(pixels, stride, width, height, 8, 8, width as i32 - 16, 48, COLOR_RAISED);
        let display_color = if self.error { 0xFFFF4444 } else { COLOR_TEXT };
        let ds = if self.display.len() > 16 {
            &self.display[self.display.len() - 16..]
        } else {
            &self.display
        };
        draw_text_on_slice(pixels, stride, width, width as i32 - 8 - ds.len() as i32 * 8, 20, ds, display_color);

        let buttons = [
            ['7', '8', '9', '/'],
            ['4', '5', '6', '*'],
            ['1', '2', '3', '-'],
            ['C', '0', '.', '+'],
            ['(', ')', '\x08', '='],
        ];

        let bw = (width as i32 - 20) / 4;
        let bh = (height as i32 - 76) / 5;
        for (row, btns) in buttons.iter().enumerate() {
            for (col, &btn) in btns.iter().enumerate() {
                let bx = 8 + col as i32 * (bw + 4);
                let by = 64 + row as i32 * (bh + 4);
                let is_op = "+-*/=".contains(btn);
                let is_c = btn == 'C' || btn == '\x08';
                let bg = if is_op { COLOR_ACCENT } else if is_c { 0xFFAA4444 } else { COLOR_HOVER };
                fill_rect_on_slice(pixels, stride, width, height, bx, by, bw, bh, bg);
                let label = if btn == '\x08' { "BS" } else { &btn.to_string() };
                draw_text_on_slice(pixels, stride, width, bx + bw/2 - label.len() as i32*4, by + bh/2 - 8, label, COLOR_TEXT);
            }
        }
    }

    fn click_char(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let buttons = [
            ['7', '8', '9', '/'],
            ['4', '5', '6', '*'],
            ['1', '2', '3', '-'],
            ['C', '0', '.', '+'],
            ['(', ')', '\x08', '='],
        ];

        let bw = (width as i32 - 20) / 4;
        let bh = (height as i32 - 76) / 5;
        for (row, btns) in buttons.iter().enumerate() {
            for (col, &btn) in btns.iter().enumerate() {
                let bx = 8 + col as i32 * (bw + 4);
                let by = 64 + row as i32 * (bh + 4);
                if x >= bx && x < bx + bw && y >= by && y < by + bh {
                    if btn == '=' {
                        self.calculate();
                    } else {
                        self.press(btn);
                    }
                    return;
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER)?;
    let width = 320;
    let height = 400;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_APPLICATION, true,
    )?;

    conn.set_nonblocking(true)?;

    let mut app = CalcApp::new();
    let mut cursor_x = (width / 2) as i32;
    let mut cursor_y = (height / 2) as i32;

    loop {
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
        let ret = unsafe { poll(&mut pfd, 1, 50) };
        if ret < 0 { continue; }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
                    t if t == LgpMessageType::PointerMotion as u16 => {
                        if let Some(ev) = parse_pointer_motion(&msg.payload) {
                            cursor_x = ev.x as i32;
                            cursor_y = ev.y as i32;
                        }
                    }
                    t if t == LgpMessageType::PointerButton as u16 => {
                        if let Some(ev) = parse_pointer_button(&msg.payload) {
                            if ev.pressed {
                                app.click_char(cursor_x, cursor_y, width, height);
                            }
                        }
                    }
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.pressed {
                                if ev.key == 0x70029 { return Ok(()); }
                                let ch = match ev.key {
                                    0x70027 => Some('0'),
                                    0x7001E => Some('1'),
                                    0x7001F => Some('2'),
                                    0x70020 => Some('3'),
                                    0x70021 => Some('4'),
                                    0x70022 => Some('5'),
                                    0x70023 => Some('6'),
                                    0x70024 => Some('7'),
                                    0x70025 => Some('8'),
                                    0x70026 => Some('9'),
                                    0x7002D => Some('-'),
                                    0x7002E => Some('='),
                                    0x7002F => Some('['),
                                    _ => None,
                                };
                                if let Some(c) = ch {
                                    if c == '=' { app.calculate(); }
                                    else { app.press(c); }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
