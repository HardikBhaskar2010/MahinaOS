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

struct AboutApp;

impl AboutApp {
    fn render(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, COLOR_BG);

        let cx = width as i32 / 2;
        draw_text_on_slice(pixels, stride, width, cx - 32, 16, "MahinaOS", COLOR_ACCENT);
        draw_text_on_slice(pixels, stride, width, cx - 28, 36, "Version 0.1", COLOR_TEXT_SEC);

        fill_rect_on_slice(pixels, stride, width, height, cx - 100, 60, 200, 1, COLOR_TEXT_SEC);

        let mut info = Vec::new();
        // Kernel
        info.push(("Kernel", std::fs::read_to_string("/proc/version")
            .ok().and_then(|s| s.split_whitespace().nth(2).map(|s| s.to_string()))
            .unwrap_or_else(|| String::from("Unknown"))));

        // Arch
        info.push(("Architecture", std::env::consts::ARCH.to_string()));

        // Hostname
        info.push(("Hostname", std::fs::read_to_string("/proc/sys/kernel/hostname")
            .ok().map(|s| s.trim().to_string())
            .unwrap_or_else(|| String::from("Unknown"))));

        // CPU
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("model name") {
                    if let Some(val) = line.split(':').nth(1) {
                        info.push(("CPU", val.trim().to_string()));
                        // Count cores
                        let cores = cpuinfo.lines()
                            .filter(|l| l.starts_with("processor"))
                            .count();
                        info.push(("Cores", format!("{}", cores)));
                    }
                    break;
                }
            }
        }

        // RAM (/proc/meminfo)
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut mem_total: u64 = 0;
            let mut mem_free: u64 = 0;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    mem_total = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
                if line.starts_with("MemFree:") {
                    mem_free = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
            let used_mb = (mem_total - mem_free) / 1024;
            let total_mb = mem_total / 1024;
            info.push(("Memory", format!("{} MB / {} MB used", used_mb, total_mb)));
        }

        // Disk
        let statvfs = unsafe {
            let mut st: libc::statvfs = std::mem::zeroed();
            libc::statvfs("/\0".as_ptr() as *const i8, &mut st);
            st
        };
        let total_gb = (statvfs.f_blocks as u64 * statvfs.f_bsize as u64) / (1024 * 1024 * 1024);
        let free_gb = (statvfs.f_bfree as u64 * statvfs.f_bsize as u64) / (1024 * 1024 * 1024);
        info.push(("Disk", format!("{} GB / {} GB free / {} GB total", total_gb - free_gb, free_gb, total_gb)));

        // Compositor
        info.push(("Compositor", String::from("lgp-compositor (Luna Graphics Protocol)")));
        info.push(("Shell", String::from("luna-shell-rs")));
        info.push(("GUI Toolkit", String::from("lunagui-rs")));
        info.push(("License", String::from("MIT")));

        let mut y = 80;
        for (key, value) in &info {
            draw_text_on_slice(pixels, stride, width, cx - 120, y, &format!("{}: {}", key, value), COLOR_TEXT);
            y += 20;
        }

        draw_text_on_slice(pixels, stride, width, 8, height as i32 - 16,
            "ESC to exit", COLOR_TEXT_SEC);
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD)?;
    let width = 360;
    let height = 240;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_APPLICATION, true,
    )?;

    conn.set_nonblocking(true)?;

    let app = AboutApp;

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
        let ret = unsafe { poll(&mut pfd, 1, 100) };
        if ret < 0 { continue; }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
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
