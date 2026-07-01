use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};

const COLOR_BG: u32 = 0xFF1E1E28;
const COLOR_SIDEBAR: u32 = 0xFF262636;
const COLOR_ACCENT: u32 = 0xFF6B7FD4;
const COLOR_TEXT: u32 = 0xFFEEEEF4;
const COLOR_TEXT_SEC: u32 = 0xFF9898AA;

struct SettingsApp {
    page: usize,
    cursor_y: i32,
}

impl SettingsApp {
    fn render(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, COLOR_BG);

        let sidebar_w: i32 = 160;
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, sidebar_w, height as i32, COLOR_SIDEBAR);

        let pages = ["Display", "Network", "Audio", "Users", "About"];
        for (i, p) in pages.iter().enumerate() {
            let py = 8 + i as i32 * 28;
            if i == self.page {
                fill_rect_on_slice(pixels, stride, width, height, 0, py, sidebar_w, 24, COLOR_ACCENT);
                draw_text_on_slice(pixels, stride, width, 8, py + 4, p, 0xFFFFFFFF);
            } else {
                draw_text_on_slice(pixels, stride, width, 8, py + 4, p, COLOR_TEXT);
            }
        }

        let content_x = sidebar_w + 16;
        let content_w = width as i32 - content_x - 16;

        match self.page {
            0 => self.render_display(pixels, stride, width, content_x, 8, content_w),
            1 => self.render_network(pixels, stride, width, content_x, 8, content_w),
            2 => self.render_audio(pixels, stride, width, content_x, 8, content_w),
            3 => self.render_users(pixels, stride, width, content_x, 8, content_w),
            4 => self.render_about(pixels, stride, width, content_x, 8, content_w),
            _ => {}
        }
    }

    fn render_display(&self, pixels: &mut [u8], stride: u32, width: u32, x: i32, y: i32, _w: i32) {
        draw_text_on_slice(pixels, stride, width, x, y, "Display Settings", COLOR_TEXT);
        draw_text_on_slice(pixels, stride, width, x, y + 24, "Resolution: 1024x768", COLOR_TEXT_SEC);
        draw_text_on_slice(pixels, stride, width, x, y + 48, "Wallpaper: /usr/share/wallpaper/", COLOR_TEXT_SEC);

        if let Ok(drm) = std::fs::read_dir("/sys/class/drm") {
            let mut ly = y + 80;
            for entry in drm.flatten() {
                let name = entry.file_name();
                let s = name.to_string_lossy();
                if s.contains("card") && !s.contains("-") {
                    // skip
                } else if s.contains("-") {
                    draw_text_on_slice(pixels, stride, width, x, ly, &format!("Output: {}", s), COLOR_TEXT_SEC);
                    ly += 20;
                }
            }
        }
    }

    fn render_network(&self, pixels: &mut [u8], stride: u32, width: u32, x: i32, y: i32, _w: i32) {
        draw_text_on_slice(pixels, stride, width, x, y, "Network Settings", COLOR_TEXT);
        if let Ok(net) = std::fs::read_to_string("/proc/net/dev") {
            for (i, line) in net.lines().enumerate().skip(2).take(6) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let rx = parts[1].parse::<u64>().unwrap_or(0) / 1024;
                    let tx = parts[9].parse::<u64>().unwrap_or(0) / 1024;
                    draw_text_on_slice(pixels, stride, width, x, y + 24 + i as i32 * 20,
                        &format!("{}  Rx:{}KB Tx:{}KB", parts[0].trim_end_matches(':'), rx, tx), COLOR_TEXT_SEC);
                }
            }
        }
    }

    fn render_audio(&self, pixels: &mut [u8], stride: u32, width: u32, x: i32, y: i32, _w: i32) {
        draw_text_on_slice(pixels, stride, width, x, y, "Audio Settings", COLOR_TEXT);
        draw_text_on_slice(pixels, stride, width, x, y + 24, "Volume: 75%", COLOR_TEXT_SEC);
        draw_text_on_slice(pixels, stride, width, x, y + 48, "Output: Default", COLOR_TEXT_SEC);
    }

    fn render_users(&self, pixels: &mut [u8], stride: u32, width: u32, x: i32, y: i32, _w: i32) {
        draw_text_on_slice(pixels, stride, width, x, y, "User Accounts", COLOR_TEXT);
        if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
            let mut ly = y + 24;
            for line in passwd.lines().take(10) {
                let user = line.split(':').next().unwrap_or("?");
                draw_text_on_slice(pixels, stride, width, x, ly, user, COLOR_TEXT_SEC);
                ly += 20;
            }
        }
    }

    fn render_about(&self, pixels: &mut [u8], stride: u32, width: u32, x: i32, y: i32, _w: i32) {
        draw_text_on_slice(pixels, stride, width, x, y, "About", COLOR_TEXT);
        let info = [
            ("OS:", "MahinaOS 0.1"),
            ("Kernel:", "Linux (custom)"),
            ("Compositor:", "lgp-compositor"),
            ("Shell:", "luna-shell-rs"),
            ("GUI Toolkit:", "lunagui-rs"),
            ("Language:", "Rust + C"),
        ];
        for (i, (k, v)) in info.iter().enumerate() {
            draw_text_on_slice(pixels, stride, width, x, y + 24 + i as i32 * 20, &format!("{} {}", k, v), COLOR_TEXT_SEC);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER)?;
    let width = conn.output_width;
    let height = conn.output_height;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_SHELL, false,
    )?;

    let app = SettingsApp { page: 0, cursor_y: 0 };
    let mut app = app;

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
        let ret = unsafe { poll(&mut pfd, 1, 16) };
        if ret < 0 { continue; }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.pressed {
                                if ev.key == 0x70052 { // Up
                                    if app.page > 0 { app.page -= 1; }
                                } else if ev.key == 0x70051 { // Down
                                    if app.page < 4 { app.page += 1; }
                                } else if ev.key == 0x70029 { // ESC
                                    return Ok(());
                                }
                            }
                        }
                    }
                    t if t == LgpMessageType::PointerButton as u16 => {
                        if let Some(ev) = parse_pointer_button(&msg.payload) {
                            if ev.pressed {
                                let px = ev.x as i32;
                                let py = ev.y as i32;
                                if px < 160 && py >= 8 {
                                    let p = (py - 8) / 28;
                                    if p >= 0 && p <= 4 { app.page = p as usize; }
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

fn _dummy() {}
