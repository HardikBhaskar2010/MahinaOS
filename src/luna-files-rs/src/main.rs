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

struct FilesApp {
    cwd: std::path::PathBuf,
    entries: Vec<String>,
    scroll: i32,
}

impl FilesApp {
    fn new() -> Self {
        let mut app = Self {
            cwd: std::path::PathBuf::from("/"),
            entries: Vec::new(),
            scroll: 0,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.entries.clear();
        if let Ok(rd) = std::fs::read_dir(&self.cwd) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    dirs.push(format!("[{}]", name));
                } else {
                    files.push(name);
                }
            }
            dirs.sort();
            files.sort();
            self.entries.extend(dirs);
            self.entries.extend(files);
        }
    }

    fn enter(&mut self, name: &str) {
        let clean = name.trim_start_matches('[').trim_end_matches(']');
        let new_path = self.cwd.join(clean);
        if new_path.is_dir() {
            self.cwd = new_path;
            self.scroll = 0;
            self.refresh();
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            if parent.as_os_str().is_empty() {
                self.cwd = std::path::PathBuf::from("/");
            } else {
                self.cwd = parent.to_path_buf();
            }
            self.scroll = 0;
            self.refresh();
        }
    }

    fn render(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, COLOR_BG);

        let cwd_str = self.cwd.to_string_lossy();
        draw_text_on_slice(pixels, stride, width, 8, 4, &format!("Location: {}", cwd_str), COLOR_ACCENT);
        fill_rect_on_slice(pixels, stride, width, height, 0, 22, width as i32, 1, COLOR_RAISED);

        draw_text_on_slice(pixels, stride, width, 8, 26, "[..] Up", COLOR_TEXT);
        let mut ly = 26;
        for (i, entry) in self.entries.iter().enumerate() {
            let py = 26 + (i as i32 + 1) * 20 - self.scroll;
            if py >= 26 && py < height as i32 - 4 {
                let color = if entry.starts_with('[') { COLOR_ACCENT } else { COLOR_TEXT };
                draw_text_on_slice(pixels, stride, width, 8, py, entry, color);
            }
            ly = py + 20;
        }
    }

    fn click_at(&mut self, y: i32) {
        let idx = ((y - 26) / 20 + self.scroll / 20) as usize;
        if idx == 0 {
            self.go_up();
        } else if idx > 0 && idx <= self.entries.len() {
            let name = self.entries[idx - 1].clone();
            self.enter(&name);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_POINTER | LGP_CAP_KEYBOARD)?;
    let width = 640;
    let height = 480;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_APPLICATION, true,
    )?;

    conn.set_nonblocking(true)?;

    let mut app = FilesApp::new();
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
                            cursor_y = ev.y as i32;
                        }
                    }
                    t if t == LgpMessageType::PointerButton as u16 => {
                        if let Some(ev) = parse_pointer_button(&msg.payload) {
                            if ev.pressed {
                                app.click_at(cursor_y);
                            }
                        }
                    }
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.pressed && ev.key == 0x70029 {
                                return Ok(());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
