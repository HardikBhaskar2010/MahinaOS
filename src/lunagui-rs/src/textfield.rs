use crate::widget::*;
use crate::canvas::Canvas;
use crate::color::{COLOR_RAISED, COLOR_TEXT, COLOR_TEXT_SECONDARY, COLOR_ACCENT, COLOR_BORDER};

pub struct TextField {
    pub base: WidgetBase,
    pub text: String,
    pub placeholder: String,
    pub cursor_pos: usize,
    pub view_offset: usize,
    pub focused: bool,
    pub password_mode: bool,
    pub max_length: usize,
}

impl TextField {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            text: String::new(),
            placeholder: String::new(),
            cursor_pos: 0,
            view_offset: 0,
            focused: false,
            password_mode: false,
            max_length: 512,
        }
    }
}

impl Widget for TextField {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; }

    fn render(&self, pixels: &mut [u8], stride: u32, _surface_w: u32, surface_h: u32) {
        let mut canvas = Canvas::from_slice(pixels, stride / 4, surface_h, stride);
        canvas.push_clip(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32);

        canvas.fill_rect(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32, COLOR_RAISED);

        let border_color = if self.focused { COLOR_ACCENT } else { COLOR_BORDER };
        canvas.draw_rect_outline(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32, border_color);

        let display = if self.password_mode {
            "*".repeat(self.text.len())
        } else {
            self.text.clone()
        };

        let visible_chars = (self.base.width as usize - 8) / 8;
        let end = std::cmp::min(display.len(), self.view_offset + visible_chars);
        let visible: String = if self.view_offset < display.len() {
            display[self.view_offset..end].to_string()
        } else {
            String::new()
        };

        canvas.draw_text(self.base.x + 4, self.base.y + (self.base.height as i32 - 16) / 2, &visible, COLOR_TEXT);

        if self.focused {
            let cursor_visible = self.cursor_pos >= self.view_offset
                && self.cursor_pos < self.view_offset + visible_chars;
            if cursor_visible {
                let cx = self.base.x + 4 + ((self.cursor_pos - self.view_offset) * 8) as i32;
                let cy = self.base.y + (self.base.height as i32 - 16) / 2;
                canvas.fill_rect(cx, cy, 1, 16, COLOR_ACCENT);
            }
        }

        if self.text.is_empty() && !self.focused && !self.placeholder.is_empty() {
            canvas.draw_text(self.base.x + 4, self.base.y + (self.base.height as i32 - 16) / 2,
                &self.placeholder, COLOR_TEXT_SECONDARY);
        }

        canvas.pop_clip();
    }

    fn hit_test(&self, x: i32, y: i32) -> bool { self.base.hit_test(x, y) }

    fn on_click(&mut self, _x: i32, _y: i32) {
        self.focused = true;
        let click_x = _x - self.base.x - 4;
        let click_char = ((click_x + 4) / 8) as usize + self.view_offset;
        self.cursor_pos = std::cmp::min(click_char, self.text.len());
    }

    fn on_key(&mut self, key: u32, pressed: bool) {
        if !pressed || !self.focused { return; }
        match key {
            0x7002A => {
                if self.cursor_pos > 0 {
                    self.text.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                    if self.view_offset > 0 && self.cursor_pos < self.view_offset {
                        self.view_offset = self.cursor_pos.saturating_sub(4);
                    }
                }
            }
            0x7004C => {
                if self.cursor_pos < self.text.len() {
                    self.text.remove(self.cursor_pos);
                }
            }
            0x70050 => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    if self.cursor_pos < self.view_offset {
                        self.view_offset = self.view_offset.saturating_sub(8);
                    }
                }
            }
            0x7004F => {
                if self.cursor_pos < self.text.len() {
                    self.cursor_pos += 1;
                    let visible_chars = (self.base.width as usize - 8) / 8;
                    if self.cursor_pos >= self.view_offset + visible_chars {
                        self.view_offset = self.cursor_pos.saturating_sub(visible_chars) + 1;
                    }
                }
            }
            0x70053 => {
                self.cursor_pos = 0;
                self.view_offset = 0;
            }
            0x70057 => {
                self.cursor_pos = self.text.len();
                let visible_chars = (self.base.width as usize - 8) / 8;
                if self.cursor_pos >= visible_chars {
                    self.view_offset = self.cursor_pos - visible_chars + 1;
                }
            }
            _ => {
                if key >= 0x04 && key <= 0x1D && !pressed {
                    // Printable ASCII from keyboard HID usage codes
                    let ch = match key {
                        0x04 => Some('a'), 0x05 => Some('b'), 0x06 => Some('c'),
                        0x07 => Some('d'), 0x08 => Some('e'), 0x09 => Some('f'),
                        0x0A => Some('g'), 0x0B => Some('h'), 0x0C => Some('i'),
                        0x0D => Some('j'), 0x0E => Some('k'), 0x0F => Some('l'),
                        0x10 => Some('m'), 0x11 => Some('n'), 0x12 => Some('o'),
                        0x13 => Some('p'), 0x14 => Some('q'), 0x15 => Some('r'),
                        0x16 => Some('s'), 0x17 => Some('t'), 0x18 => Some('u'),
                        0x19 => Some('v'), 0x1A => Some('w'), 0x1B => Some('x'),
                        0x1C => Some('y'), 0x1D => Some('z'),
                        0x1E => Some('1'), 0x1F => Some('2'),
                        0x20 => Some('3'), 0x21 => Some('4'), 0x22 => Some('5'),
                        0x23 => Some('6'), 0x24 => Some('7'), 0x25 => Some('8'),
                        0x26 => Some('9'), 0x27 => Some('0'),
                        0x28 => Some('\n'), 0x29 => Some('\x1b'),
                        0x2A => Some('\x08'), 0x2B => Some('\t'),
                        0x2C => Some(' '), 0x2D => Some('-'), 0x2E => Some('='),
                        0x2F => Some('['), 0x30 => Some(']'), 0x31 => Some('\\'),
                        0x33 => Some(';'), 0x34 => Some('\''), 0x35 => Some('`'),
                        0x36 => Some(','), 0x37 => Some('.'), 0x38 => Some('/'),
                        _ => None,
                    };
                    if let Some(c) = ch {
                        if self.text.len() < self.max_length {
                            self.text.insert(self.cursor_pos, c);
                            self.cursor_pos += 1;
                            let visible_chars = (self.base.width as usize - 8) / 8;
                            if self.cursor_pos >= self.view_offset + visible_chars {
                                self.view_offset = self.cursor_pos.saturating_sub(visible_chars) + 1;
                            }
                        }
                    }
                }
            }
        }
    }
}
