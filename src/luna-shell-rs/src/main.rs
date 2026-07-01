use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};

fn render_wallpaper(pixels: &mut [u8], stride: u32, width: u32, height: u32) {
    for y in 0..height {
        let t = y as f64 / height as f64;
        let r = ((0.12 * (1.0 - t) + 0.08 * t) * 255.0) as u32;
        let g = ((0.12 * (1.0 - t) + 0.08 * t) * 255.0) as u32;
        let b = ((0.16 * (1.0 - t) + 0.10 * t) * 255.0) as u32;
        let color = 0xFF000000 | (r << 16) | (g << 8) | b;
        for x in 0..width {
            let off = (y * stride + x * 4) as usize;
            pixels[off..off + 4].copy_from_slice(&color.to_le_bytes());
        }
    }
    let logo = "MahinaOS";
    let cx = (width as i32 - logo.len() as i32 * 8) / 2;
    let cy = height as i32 / 2 - 8;
    draw_text_on_slice(pixels, stride, width, cx, cy, logo, 0xFF6B7FD4);
}

fn render_overlay(pixels: &mut [u8], stride: u32, width: u32, height: u32, clock_h: u8, clock_m: u8) {
    const TOPBAR_H: u32 = 28;
    const TOPBAR_COLOR: u32 = 0xEE0A0A14;
    const ACCENT: u32 = 0xFFE03E8A;
    const TEXT_COLOR: u32 = 0xFFD8D8E8;

    fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, TOPBAR_H as i32, TOPBAR_COLOR);
    fill_rect_on_slice(pixels, stride, width, height, 0, TOPBAR_H as i32 - 1, width as i32, 1, ACCENT);

    draw_text_on_slice(pixels, stride, width, 8, 6, "MahinaOS", ACCENT);
    draw_text_on_slice(pixels, stride, width, 8 + 7 * 8, 6, "|", TEXT_COLOR);

    let ws = "[1]  2  3 ";
    draw_text_on_slice(pixels, stride, width, 8 + 9 * 8, 6, ws, TEXT_COLOR);

    let clock_str = format!("{:02}:{:02}", clock_h, clock_m);
    let cx = width as i32 - clock_str.len() as i32 * 8 - 8;
    draw_text_on_slice(pixels, stride, width, cx, 6, &clock_str, TEXT_COLOR);

    const DOCK_H: u32 = 64;
    const DOCK_BOTTOM: u32 = 4;
    const DOCK_BG: u32 = 0xB008090E;

    let dock_y = height - DOCK_H - DOCK_BOTTOM;
    let dock_w = if width < 480 { width } else { 480 };
    let dock_x = (width - dock_w) / 2;

    fill_rect_on_slice(pixels, stride, width, height, dock_x as i32, dock_y as i32, dock_w as i32, DOCK_H as i32, DOCK_BG);

    let apps = [
        ("Settngs", 0xFFE03E8A),
        ("Files", 0xFFD8D8E8),
        ("Calc", 0xFFD8D8E8),
        ("Text", 0xFFD8D8E8),
        ("Tasks", 0xFFD8D8E8),
        ("About", 0xFFD8D8E8),
    ];

    let tile_w = dock_w / apps.len() as u32;
    let tile_h = DOCK_H - 8;
    let tile_base_y = dock_y + 4;

    for (i, (name, tc)) in apps.iter().enumerate() {
        let tx = dock_x + i as u32 * tile_w;
        let nx = tx + tile_w / 2 - (name.len() as u32 * 4);
        let ny = tile_base_y + tile_h / 2 - 8;
        draw_text_on_slice(pixels, stride, width, nx as i32, ny as i32, name, *tc);
    }
}

fn main() -> std::io::Result<()> {
    let caps = LGP_CAP_LAYER_SHELL | LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER;
    let mut conn = LgpConnection::connect(caps)?;

    let width = conn.output_width;
    let height = conn.output_height;

    let mut wallpaper = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_WALLPAPER, false,
    )?;

    let mut overlay = LgpSurface::create(
        &mut conn, LGP_SURFACE_SHELL_SURFACE, 0, 0,
        width, height, LGP_LAYER_SHELL, true,
    )?;

    let mut clock_h: u8 = 0;
    let mut clock_m: u8 = 0;
    let mut frame_count: u64 = 0;

    loop {
        let wp_pixels = wallpaper.pixels();
        render_wallpaper(wp_pixels, width * 4, width, height);
        wallpaper.commit(&mut conn)?;

        let ov_pixels = overlay.pixels();
        render_overlay(ov_pixels, width * 4, width, height, clock_h, clock_m);
        overlay.commit(&mut conn)?;

        frame_count += 1;
        if frame_count % 10 == 0 {
            clock_m = (clock_m + 1) % 60;
            if clock_m == 0 {
                clock_h = (clock_h + 1) % 24;
            }
        }

        let mut pfd = pollfd {
            fd: conn.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        loop {
            let ret = unsafe { poll(&mut pfd, 1, 100) };
            if ret < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(std::io::Error::last_os_error());
            }
            break;
        }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.key == 0x70029 && !ev.pressed {
                                return Ok(()); // ESC quits
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn _launcher_hide() {}
