/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * dock.rs — Bottom app launcher dock.
 * Apps are launched via fork()/exec() directly, matching how desktop DEs work.
 * Running app indicators shown as accent dots below icons.
 */

use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};

const DOCK_H: i32 = 64;
const DOCK_BOTTOM_MARGIN: i32 = 8;
const TILE_W: i32 = 72;
const TILE_H: i32 = DOCK_H - 8;
const BG: u32 = 0xCC080C14;
const TILE_HOVER: u32 = 0xDD151E2C;
const TILE_ACTIVE: u32 = 0xFF1A2035;
const ACCENT: u32 = 0xFFE03E8A;
const TEXT: u32 = 0xFFD8D8E8;

pub struct DockApp {
    pub name: &'static str,
    pub binary: &'static str,
    pub icon_color: u32,
    pub running: bool,
    pub pid: Option<u32>,
}

pub struct Dock {
    pub apps: Vec<DockApp>,
    hover_idx: Option<usize>,
    pub visible: bool,
}

impl Dock {
    pub fn new() -> Self {
        Self {
            apps: vec![
                DockApp { name: "Terminal", binary: "/usr/bin/luna-terminal", icon_color: 0xFF3EE09A, running: false, pid: None },
                DockApp { name: "Files",    binary: "/usr/bin/files-rs",      icon_color: 0xFF6B7FD4, running: false, pid: None },
                DockApp { name: "Settings", binary: "/usr/bin/settings-rs",   icon_color: 0xFFE03E8A, running: false, pid: None },
                DockApp { name: "Calc",     binary: "/usr/bin/calc-rs",       icon_color: 0xFFE09A3E, running: false, pid: None },
                DockApp { name: "Text",     binary: "/usr/bin/text-rs",       icon_color: 0xFF9A3EE0, running: false, pid: None },
                DockApp { name: "Tasks",    binary: "/usr/bin/tasks-rs",      icon_color: 0xFF3EE0C0, running: false, pid: None },
                DockApp { name: "About",    binary: "/usr/bin/about-rs",      icon_color: 0xFF9898AA, running: false, pid: None },
            ],
            hover_idx: None,
            visible: true,
        }
    }

    pub fn dock_rect(&self, sw: u32) -> (i32, i32, i32, i32) {
        let dock_w = TILE_W * self.apps.len() as i32 + 8;
        let dock_x = (sw as i32 - dock_w) / 2;
        let dock_y = sw as i32; // placeholder, caller uses sh
        (dock_x, dock_w, DOCK_H, DOCK_BOTTOM_MARGIN)
    }

    fn dock_y(&self, sh: u32) -> i32 {
        sh as i32 - DOCK_H - DOCK_BOTTOM_MARGIN
    }

    fn dock_x(&self, sw: u32) -> i32 {
        let dock_w = TILE_W * self.apps.len() as i32 + 8;
        (sw as i32 - dock_w) / 2
    }

    fn dock_w(&self) -> i32 {
        TILE_W * self.apps.len() as i32 + 8
    }

    pub fn on_pointer_motion(&mut self, x: f64, y: f64, sw: u32, sh: u32) {
        let dy = self.dock_y(sh);
        let dx = self.dock_x(sw);
        let dw = self.dock_w();

        if (y as i32) >= dy && (y as i32) < dy + DOCK_H
           && (x as i32) >= dx && (x as i32) < dx + dw {
            let rel = (x as i32 - dx - 4) / TILE_W;
            if rel >= 0 && (rel as usize) < self.apps.len() {
                self.hover_idx = Some(rel as usize);
                return;
            }
        }
        self.hover_idx = None;
    }

    /// Returns the index of the app that was clicked (if any).
    pub fn on_click(&self, x: f64, y: f64, sw: u32, sh: u32) -> Option<usize> {
        let dy = self.dock_y(sh);
        let dx = self.dock_x(sw);
        let dw = self.dock_w();

        if (y as i32) >= dy && (y as i32) < dy + DOCK_H
           && (x as i32) >= dx && (x as i32) < dx + dw {
            let rel = (x as i32 - dx - 4) / TILE_W;
            if rel >= 0 && (rel as usize) < self.apps.len() {
                return Some(rel as usize);
            }
        }
        None
    }

    /// Launch an app by index using fork/exec (matching GNOME/KDE behavior).
    /// Returns the child PID on success.
    pub fn launch(&mut self, idx: usize) -> Option<u32> {
        if idx >= self.apps.len() { return None; }
        let binary = self.apps[idx].binary;

        // Use fork() + exec() — same as what gnome-shell / plasmashell do under the hood
        // The shell doesn't need D-Bus in our system since we don't have sandboxing.
        let pid = unsafe {
            let pid = libc::fork();
            if pid < 0 {
                // Fork failed
                return None;
            }
            if pid == 0 {
                // Child: exec the binary
                // Ensure we don't inherit shell FDs that would block the compositor
                for fd in 3..256i32 { libc::close(fd); }

                // Set the binary path as argv[0]
                let path = std::ffi::CString::new(binary).unwrap_or_default();
                let null = std::ffi::CString::new("").unwrap();
                let args = [path.as_ptr(), std::ptr::null()];
                libc::execv(path.as_ptr(), args.as_ptr());
                // exec failed — exit child
                libc::_exit(127);
            }
            pid as u32
        };

        self.apps[idx].running = true;
        self.apps[idx].pid = Some(pid);
        Some(pid)
    }

    /// Reap any zombie children and update running state.
    pub fn reap_children(&mut self) {
        for app in self.apps.iter_mut() {
            if let Some(pid) = app.pid {
                let mut status: i32 = 0;
                let ret = unsafe {
                    libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG)
                };
                if ret == pid as libc::pid_t {
                    app.running = false;
                    app.pid = None;
                }
            }
        }
    }

    pub fn render(&self, pixels: &mut [u8], stride: u32, sw: u32, sh: u32) {
        if !self.visible { return; }

        let dy = self.dock_y(sh);
        let dx = self.dock_x(sw);
        let dw = self.dock_w();

        // Dock background — glassmorphic dark panel
        fill_rect_on_slice(pixels, stride, sw, sh, dx, dy, dw, DOCK_H, BG);

        // Top border glow
        fill_rect_on_slice(pixels, stride, sw, sh, dx, dy, dw, 1, 0x40E03E8A);

        // App tiles
        for (i, app) in self.apps.iter().enumerate() {
            let tx = dx + 4 + i as i32 * TILE_W;
            let ty = dy + 4;
            let is_hover = self.hover_idx == Some(i);

            // Tile background
            let tile_bg = if app.running {
                TILE_ACTIVE
            } else if is_hover {
                TILE_HOVER
            } else {
                0x00000000
            };
            if tile_bg != 0 {
                fill_rect_on_slice(pixels, stride, sw, sh, tx, ty, TILE_W - 4, TILE_H, tile_bg);
            }

            // App icon: a colored rounded square (approximated)
            let icon_size = 32i32;
            let icon_x = tx + (TILE_W - 4 - icon_size) / 2;
            let icon_y = ty + 2;
            fill_rect_on_slice(pixels, stride, sw, sh, icon_x, icon_y, icon_size, icon_size, app.icon_color);
            // Inner highlight for depth
            fill_rect_on_slice(pixels, stride, sw, sh, icon_x + 2, icon_y + 2, icon_size - 4, 8, 0x30FFFFFF);

            // App name
            let name_x = tx + (TILE_W - 4 - app.name.len() as i32 * 8) / 2;
            let name_y = ty + TILE_H - 18;
            let name_color = if is_hover { 0xFFFFFFFF } else { TEXT };
            draw_text_on_slice(pixels, stride, sw, name_x.max(tx), name_y, app.name, name_color);

            // Running indicator dot
            if app.running {
                let dot_x = tx + (TILE_W - 4) / 2 - 2;
                let dot_y = dy + DOCK_H - 6;
                fill_rect_on_slice(pixels, stride, sw, sh, dot_x, dot_y, 4, 3, ACCENT);
            }
        }
    }
}
