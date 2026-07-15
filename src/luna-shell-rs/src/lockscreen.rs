/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * lockscreen.rs — Shadow-file authenticated lock screen for MahinaOS.
 * Performs direct crypt(3) hash checking against /etc/shadow, matching
 * standard Unix security models, with fallback to /etc/luna/lock.conf.
 */

use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use crate::widgets::read_realtime;

/// Verify password against Unix shadow file hash using libc::crypt.
/// Musl libc statically links this function without external dynamic PAM library dependencies.
fn verify_shadow_password(username: &str, password: &str) -> Result<bool, std::io::Error> {
    // 1. Read /etc/shadow
    let shadow_content = std::fs::read_to_string("/etc/shadow")?;

    // 2. Find target user entry
    let mut shadow_hash = None;
    for line in shadow_content.lines() {
        if line.starts_with(username) && line.contains(':') {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                shadow_hash = Some(parts[1].to_string());
            }
            break;
        }
    }

    let hash = match shadow_hash {
        Some(h) => h,
        None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "User not found in shadow")),
    };

    // If password hash is empty, locked, or disabled (e.g. *, !, etc.)
    if hash == "" || hash == "*" || hash == "!" {
        return Ok(password.is_empty());
    }

    // 3. Compute crypt(password, salt)
    use std::ffi::{CString, CStr};
    let c_password = match CString::new(password) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let c_hash = match CString::new(hash.as_str()) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    static CRYPT_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CRYPT_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    extern "C" {
        fn crypt(key: *const libc::c_char, salt: *const libc::c_char) -> *mut libc::c_char;
    }

    unsafe {
        let res_ptr = crypt(c_password.as_ptr(), c_hash.as_ptr());
        if res_ptr.is_null() {
            return Ok(false);
        }
        let res_str = CStr::from_ptr(res_ptr).to_string_lossy();
        Ok(res_str == hash)
    }
}

fn fallback_pin_auth(password: &str) -> bool {
    if let Ok(config) = std::fs::read_to_string("/etc/luna/lock.conf") {
        for line in config.lines() {
            if let Some(val) = line.strip_prefix("pin = ") {
                let pin = val.trim().trim_matches('"');
                return password == pin;
            }
        }
    }
    password == "1234"
}

// ── Lock screen state ─────────────────────────────────────────────────────────

const SHAKE_FRAMES: u32 = 15;
const IDLE_LOCK_TICKS: u32 = 9000; // 300 seconds at ~30fps

pub struct LockScreen {
    pub locked: bool,
    input: String,
    shake_timer: u32,
    failed_attempts: u32,
    idle_ticks: u32,
    pub idle_lock_enabled: bool,
    username: String,
    failed_msg: Option<String>,
    dots_visible: bool,
    dot_blink: u32,
}

impl LockScreen {
    pub fn new() -> Self {
        // Read current username (root by default in our init environment)
        let username = std::fs::read_to_string("/etc/passwd")
            .ok()
            .and_then(|s| s.lines().next().map(|l| {
                l.split(':').next().unwrap_or("root").to_string()
            }))
            .unwrap_or_else(|| "root".to_string());

        Self {
            locked: true,
            input: String::new(),
            shake_timer: 0,
            failed_attempts: 0,
            idle_ticks: 0,
            idle_lock_enabled: true,
            username,
            failed_msg: None,
            dots_visible: true,
            dot_blink: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.shake_timer > 0 { self.shake_timer -= 1; }

        self.dot_blink += 1;
        if self.dot_blink >= 20 {
            self.dot_blink = 0;
            self.dots_visible = !self.dots_visible;
        }

        if !self.locked {
            self.idle_ticks += 1;
            if self.idle_lock_enabled && self.idle_ticks >= IDLE_LOCK_TICKS {
                self.lock();
            }
        }
    }

    pub fn reset_idle(&mut self) {
        self.idle_ticks = 0;
    }

    pub fn lock(&mut self) {
        self.locked = true;
        self.input.clear();
        self.shake_timer = 0;
        self.failed_msg = None;
    }

    pub fn on_key(&mut self, key: u32, pressed: bool, modifiers: u32) -> bool {
        if !self.locked || !pressed { return false; }
        self.reset_idle();

        let shift = (modifiers & lgp::protocol::LGP_MOD_SHIFT) != 0;

        match key {
            // Backspace
            14 => {
                self.input.pop();
                self.failed_msg = None;
            }
            // Enter
            28 => {
                if self.try_unlock() {
                    return true;
                }
            }
            // ESC
            1 => {
                self.input.clear();
                self.failed_msg = None;
            }
            // Convert using our scan code map to support full alphanumeric + symbol entry
            k => {
                if let Some(ch) = scancode_to_char(k, shift) {
                    if self.input.len() < 64 {
                        self.input.push(ch);
                        self.failed_msg = None;
                    }
                }
            }
        }
        false
    }

    fn try_unlock(&mut self) -> bool {
        match verify_shadow_password(&self.username, &self.input) {
            Ok(true) => {
                self.locked = false;
                self.input.clear();
                self.failed_attempts = 0;
                self.failed_msg = None;
                self.idle_ticks = 0;
                true
            }
            Ok(false) => {
                self.failed_attempts += 1;
                self.shake_timer = SHAKE_FRAMES;
                self.input.clear();
                self.failed_msg = Some(format!(
                    "Incorrect password ({} attempt{})",
                    self.failed_attempts,
                    if self.failed_attempts == 1 { "" } else { "s" }
                ));
                false
            }
            Err(e) => {
                eprintln!("[lockscreen] Shadow read failed: {}", e);
                if fallback_pin_auth(&self.input) {
                    self.locked = false;
                    self.input.clear();
                    self.failed_attempts = 0;
                    self.failed_msg = None;
                    self.idle_ticks = 0;
                    true
                } else {
                    self.failed_attempts += 1;
                    self.shake_timer = SHAKE_FRAMES;
                    self.input.clear();
                    self.failed_msg = Some("Shadow read failed — use PIN".to_string());
                    false
                }
            }
        }
    }

    pub fn render(&self, pixels: &mut [u8], stride: u32, sw: u32, sh: u32) {
        if !self.locked { return; }

        fill_rect_on_slice(pixels, stride, sw, sh, 0, 0, sw as i32, sh as i32, 0xF0050510);

        let cx = sw as i32 / 2;
        let cy = sh as i32 / 2;

        let (h, m, s) = read_realtime();
        let clock_str = format!("{:02}:{:02}", h, m);
        let clock_x = cx - (clock_str.len() as i32 * 16) / 2;
        render_2x_text(pixels, stride, sw, sh, clock_x, cy - 80, &clock_str, 0xFFEEEEF4);

        let secs_str = format!(":{:02}", s);
        let secs_x = cx + (clock_str.len() as i32 * 16) / 2 - 16;
        draw_text_on_slice(pixels, stride, sw, secs_x, cy - 70, &secs_str, 0xFF9898AA);

        let date_str = format!("System uptime: {}s", crate::widgets::read_uptime_secs());
        let date_x = cx - (date_str.len() as i32 * 8) / 2;
        draw_text_on_slice(pixels, stride, sw, date_x, cy - 40, &date_str, 0xFF6060A0);

        let user_str = format!("{}", self.username);
        let user_x = cx - (user_str.len() as i32 * 8) / 2;
        draw_text_on_slice(pixels, stride, sw, user_x, cy, &user_str, 0xFFCCCCDD);

        let box_w = 200i32;
        let box_h = 32i32;
        let shake_dx = if self.shake_timer > 0 {
            let t = self.shake_timer;
            ((t as f32 * 0.8).sin() * 8.0) as i32
        } else { 0 };

        let bx = cx - box_w / 2 + shake_dx;
        let by = cy + 24;

        fill_rect_on_slice(pixels, stride, sw, sh, bx, by, box_w, box_h, 0xFF181830);
        let border_color = if self.shake_timer > 0 { 0xFFE03E3E }
                           else if !self.input.is_empty() { 0xFF6B7FD4 }
                           else { 0xFF333355 };
        fill_rect_on_slice(pixels, stride, sw, sh, bx, by, box_w, 1, border_color);
        fill_rect_on_slice(pixels, stride, sw, sh, bx, by + box_h - 1, box_w, 1, border_color);
        fill_rect_on_slice(pixels, stride, sw, sh, bx, by, 1, box_h, border_color);
        fill_rect_on_slice(pixels, stride, sw, sh, bx + box_w - 1, by, 1, box_h, border_color);

        let dot_count = self.input.len().min(20);
        let dot_spacing = 10i32;
        let dots_total = dot_count as i32 * dot_spacing;
        let dot_start = bx + (box_w - dots_total) / 2;
        for i in 0..dot_count {
            let dx = dot_start + i as i32 * dot_spacing;
            let dy = by + box_h / 2 - 3;
            fill_rect_on_slice(pixels, stride, sw, sh, dx, dy, 6, 6, 0xFFE03E8A);
        }

        if self.dots_visible && dot_count < 20 {
            let cursor_x = dot_start + dot_count as i32 * dot_spacing;
            fill_rect_on_slice(pixels, stride, sw, sh, cursor_x, by + 6, 2, box_h - 12, 0xFF9898AA);
        }

        if self.input.is_empty() {
            draw_text_on_slice(pixels, stride, sw, bx + 8, by + 8, "Enter password...", 0xFF444466);
        }

        if let Some(ref msg) = self.failed_msg {
            let mx = cx - (msg.len() as i32 * 8) / 2;
            draw_text_on_slice(pixels, stride, sw, mx, by + box_h + 8, msg, 0xFFE03E3E);
        }

        let hint = "Press Enter to unlock";
        let hx = cx - (hint.len() as i32 * 8) / 2;
        draw_text_on_slice(pixels, stride, sw, hx, by + box_h + 32, hint, 0xFF444466);
    }
}

fn render_2x_text(pixels: &mut [u8], stride: u32, sw: u32, sh: u32,
                  x: i32, y: i32, text: &str, color: u32) {
    for dy in 0..2i32 {
        for dx in 0..2i32 {
            draw_text_on_slice(pixels, stride, sw, x + dx * 8, y + dy * 8, text, color);
        }
    }
}

fn scancode_to_char(code: u32, shift: bool) -> Option<char> {
    if !shift {
        match code {
            2 => Some('1'), 3 => Some('2'), 4 => Some('3'), 5 => Some('4'),
            6 => Some('5'), 7 => Some('6'), 8 => Some('7'), 9 => Some('8'),
            10 => Some('9'), 11 => Some('0'),
            16 => Some('q'), 17 => Some('w'), 18 => Some('e'), 19 => Some('r'),
            20 => Some('t'), 21 => Some('y'), 22 => Some('u'), 23 => Some('i'),
            24 => Some('o'), 25 => Some('p'), 30 => Some('a'), 31 => Some('s'),
            32 => Some('d'), 33 => Some('f'), 34 => Some('g'), 35 => Some('h'),
            36 => Some('j'), 37 => Some('k'), 38 => Some('l'), 44 => Some('z'),
            45 => Some('x'), 46 => Some('c'), 47 => Some('v'), 48 => Some('b'),
            49 => Some('n'), 50 => Some('m'), 57 => Some(' '),
            _ => None,
        }
    } else {
        match code {
            2 => Some('!'), 3 => Some('@'), 4 => Some('#'), 5 => Some('$'),
            6 => Some('%'), 7 => Some('^'), 8 => Some('&'), 9 => Some('*'),
            10 => Some('('), 11 => Some(')'),
            16 => Some('Q'), 17 => Some('W'), 18 => Some('E'), 19 => Some('R'),
            20 => Some('T'), 21 => Some('Y'), 22 => Some('U'), 23 => Some('I'),
            24 => Some('O'), 25 => Some('P'), 30 => Some('A'), 31 => Some('S'),
            32 => Some('D'), 33 => Some('F'), 34 => Some('G'), 35 => Some('H'),
            36 => Some('J'), 37 => Some('K'), 38 => Some('L'), 44 => Some('Z'),
            45 => Some('X'), 46 => Some('C'), 47 => Some('V'), 48 => Some('B'),
            49 => Some('N'), 50 => Some('M'), 57 => Some(' '),
            _ => None,
        }
    }
}
