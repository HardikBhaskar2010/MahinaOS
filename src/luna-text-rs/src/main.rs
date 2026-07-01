use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};

const COLOR_BG: u32 = 0xFF1E1E28;
const COLOR_TEXT: u32 = 0xFFEEEEF4;
const COLOR_ACCENT: u32 = 0xFF6B7FD4;
const COLOR_CURSOR: u32 = 0xFF8A9FFF;
const COLOR_STATUS_BG: u32 = 0xFF262636;

struct TextEditor {
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    scroll_x: i32,
    scroll_y: i32,
    mode: EditorMode,
    modified: bool,
    filename: String,
}

enum EditorMode {
    Normal,
    Insert,
}

impl TextEditor {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            mode: EditorMode::Insert,
            modified: false,
            filename: String::from("[Untitled]"),
        }
    }

    fn insert_char(&mut self, ch: char) {
        let line = &mut self.lines[self.cursor_y];
        if self.cursor_x <= line.len() {
            line.insert(self.cursor_x, ch);
            self.cursor_x += 1;
            self.modified = true;
        }
    }

    fn backspace(&mut self) {
        if self.cursor_x > 0 {
            self.lines[self.cursor_y].remove(self.cursor_x - 1);
            self.cursor_x -= 1;
            self.modified = true;
        } else if self.cursor_y > 0 {
            let prev_len = self.lines[self.cursor_y - 1].len();
            let rest = self.lines.remove(self.cursor_y);
            self.lines[self.cursor_y - 1] += &rest;
            self.cursor_y -= 1;
            self.cursor_x = prev_len;
            self.modified = true;
        }
    }

    fn newline(&mut self) {
        let line = &mut self.lines[self.cursor_y];
        let rest = line.split_off(self.cursor_x);
        self.lines.insert(self.cursor_y + 1, rest);
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.modified = true;
    }

    fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.lines[self.cursor_y].len();
        }
    }

    fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_y].len();
        if self.cursor_x < line_len {
            self.cursor_x += 1;
        } else if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            self.cursor_x = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
        }
    }

    fn move_down(&mut self) {
        if self.cursor_y + 1 < self.lines.len() {
            self.cursor_y += 1;
            self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
        }
    }

    fn render(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, COLOR_BG);

        // Status bar
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, 20, COLOR_STATUS_BG);
        let mode_str = match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
        };
        draw_text_on_slice(pixels, stride, width, 4, 2, &format!("{} {}  {}:{}",
            mode_str, self.filename, self.cursor_y + 1, self.cursor_x + 1), COLOR_ACCENT);
        if self.modified {
            draw_text_on_slice(pixels, stride, width, width as i32 - 60, 2, "[Modified]", 0xFFFFAA00);
        }

        // Text area
        let text_y = 22;
        let visible_lines = (height as i32 - 22) / 20;
        for i in 0..visible_lines {
            let ly = self.scroll_y + i as i32;
            if ly >= 0 && ly < self.lines.len() as i32 {
                let line = &self.lines[ly as usize];
                let visible = if self.scroll_x > 0 && (self.scroll_x as usize) < line.len() {
                    &line[self.scroll_x as usize..]
                } else if self.scroll_x > 0 {
                    ""
                } else {
                    line
                };
                let max_visible = ((width as i32 - 8) / 8) as usize;
                let display = if visible.len() > max_visible {
                    &visible[..max_visible]
                } else {
                    visible
                };
                draw_text_on_slice(pixels, stride, width, 4, text_y + i * 20, display, COLOR_TEXT);
            }
        }

        // Cursor
        if self.cursor_y >= self.scroll_y as usize && self.cursor_y < self.scroll_y as usize + visible_lines as usize {
            let cx = 4 + (self.cursor_x as i32 - self.scroll_x) * 8;
            let cy = text_y + (self.cursor_y as i32 - self.scroll_y) * 20;
            fill_rect_on_slice(pixels, stride, width, height, cx, cy, 1, 16, COLOR_CURSOR);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER)?;
    let width = 600;
    let height = 400;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_APPLICATION, true,
    )?;

    conn.set_nonblocking(true)?;

    let mut editor = TextEditor::new();

    loop {
        {
            let pixels = surface.pixels();
            editor.render(pixels, width * 4, width, height);
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
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if !ev.pressed { continue; }

                            match editor.mode {
                                EditorMode::Insert => {
                                    if ev.key == 0x70029 { return Ok(()); }
                                    if ev.key == 0x70028 {
                                        editor.newline();
                                    } else if ev.key == 0x7002A {
                                        editor.backspace();
                                    } else if ev.key == 0x70050 { editor.move_left(); }
                                    else if ev.key == 0x7004F { editor.move_right(); }
                                    else if ev.key == 0x70052 { editor.move_up(); }
                                    else if ev.key == 0x70051 { editor.move_down(); }
                                    else if ev.key == 0x7004C {
                                        let line = &mut editor.lines[editor.cursor_y];
                                        if editor.cursor_x < line.len() {
                                            line.remove(editor.cursor_x);
                                            editor.modified = true;
                                        }
                                    }
                                    else {
                                        let ch = match ev.key {
                                            0x70004..=0x7001D => {
                                                let base = (ev.key - 0x70004) as u8;
                                                (if ev.modifiers & 0x2 != 0 { base + b'A' } else { base + b'a' }) as char
                                            }
                                            0x70027 => '0', 0x7001E => '1', 0x7001F => '2',
                                            0x70020 => '3', 0x70021 => '4', 0x70022 => '5',
                                            0x70023 => '6', 0x70024 => '7', 0x70025 => '8',
                                            0x70026 => '9',
                                            0x7002C => ' ', 0x7002D => '-', 0x7002E => '=',
                                            0x7002F => '[', 0x70030 => ']', 0x70031 => '\\',
                                            0x70033 => ';', 0x70034 => '\'', 0x70035 => '`',
                                            0x70036 => ',', 0x70037 => '.', 0x70038 => '/',
                                            _ => continue,
                                        };
                                        editor.insert_char(ch);
                                    }
                                }
                                EditorMode::Normal => {
                                    if ev.key == 0x70029 { return Ok(()); }
                                    if ev.key == 0x70028 { editor.mode = EditorMode::Insert; }
                                    else if ev.key == 0x70050 { editor.move_left(); }
                                    else if ev.key == 0x7004F { editor.move_right(); }
                                    else if ev.key == 0x70052 { editor.move_up(); }
                                    else if ev.key == 0x70051 { editor.move_down(); }
                                    else if ev.key == 0x7002A {
                                        editor.backspace();
                                    } else if ev.key == 0x7002C {
                                        editor.mode = EditorMode::Insert;
                                    }
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
