/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * shell.rs — Central orchestrator for the MahinaOS Desktop Shell.
 * Manages the layout, inputs, background, lockscreen, dock, topbar, widgets, and notifications.
 */

use std::collections::HashMap;
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use crate::keybinds::{dispatch, ShellAction, keys};
use crate::notifications::NotificationManager;
use crate::wallpaper::WallpaperEngine;
use crate::widgets::DesktopWidgets;
use crate::wm::{WmManager, SnapDir};
use crate::launcher::Launcher;
use crate::dock::Dock;
use crate::taskbar::{Taskbar, TaskbarAction};
use crate::lockscreen::LockScreen;

pub struct ShellState {
    pub lockscreen: LockScreen,
    pub wm: WmManager,
    pub wallpaper: WallpaperEngine,
    pub widgets: DesktopWidgets,
    pub launcher: Launcher,
    pub dock: Dock,
    pub taskbar: Taskbar,
    pub notifications: NotificationManager,
    pub screen_w: u32,
    pub screen_h: u32,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

impl ShellState {
    pub fn new(sw: u32, sh: u32) -> Self {
        Self {
            lockscreen: LockScreen::new(),
            wm: WmManager::new(sw, sh),
            wallpaper: WallpaperEngine::new(sw, sh),
            widgets: DesktopWidgets::new(),
            launcher: Launcher::new(),
            dock: Dock::new(),
            taskbar: Taskbar::new(),
            notifications: NotificationManager::new(),
            screen_w: sw,
            screen_h: sh,
            cursor_x: (sw / 2) as f64,
            cursor_y: (sh / 2) as f64,
        }
    }

    pub fn tick(&mut self, conn: &mut LgpConnection) {
        self.lockscreen.tick();
        self.wallpaper.tick();
        self.widgets.tick();
        self.notifications.tick();
        self.dock.reap_children();

        // Update widget active workspace state
        self.widgets.set_workspace(self.wm.active_workspace, self.wm.workspace_count());

        // Auto-show/hide dock based on fullscreen window presence
        let has_fullscreen = self.wm.windows.values()
            .any(|w| w.workspace == self.wm.active_workspace && w.state == crate::wm::WmWindowState::Fullscreen);
        self.dock.visible = !has_fullscreen;
    }

    pub fn handle_keyboard(&mut self, key: u32, modifiers: u32, pressed: bool, conn: &mut LgpConnection) {
        self.lockscreen.reset_idle();

        if self.lockscreen.locked {
            if self.lockscreen.on_key(key, pressed, modifiers) {
                self.notifications.push_success("Session Unlocked via PAM");
            }
            return;
        }

        if self.launcher.visible {
            if let Some(app) = self.launcher.on_key(key, pressed) {
                let name = app.name;
                let binary = app.binary;
                self.launch_app_by_binary(name, binary);
            }
            return;
        }

        // Global shortcuts dispatch
        match dispatch(key, modifiers) {
            ShellAction::SwitchWorkspace(idx) => {
                self.wm.switch_workspace(idx, conn);
                self.notifications.push_info(format!("Workspace {}", idx + 1));
            }
            ShellAction::MoveWindowToWorkspace(idx) => {
                self.wm.move_focused_to_workspace(idx, conn);
                self.notifications.push_info(format!("Window moved to Workspace {}", idx + 1));
            }
            ShellAction::OpenTerminal => {
                self.launch_app_by_binary("Terminal", "/usr/bin/luna-terminal");
            }
            ShellAction::OpenLauncher => {
                self.launcher.visible = !self.launcher.visible;
                if self.launcher.visible {
                    self.launcher.reset();
                }
            }
            ShellAction::CloseWindow => {
                if let Some(sid) = self.wm.focused_surface {
                    let _ = lgp::wm::WmState::set_state(conn, sid, LGP_WM_STATE_HIDDEN);
                    self.wm.on_surface_destroyed(sid, conn);
                    self.notifications.push_info("Window closed");
                }
            }
            ShellAction::ToggleFloating => {
                self.wm.toggle_floating(conn);
            }
            ShellAction::ToggleFullscreen => {
                self.wm.toggle_fullscreen(conn);
            }
            ShellAction::FocusNext => {
                self.wm.focus_next(conn);
            }
            ShellAction::FocusPrev => {
                self.wm.focus_prev(conn);
            }
            ShellAction::SnapLeft => {
                self.wm.snap(SnapDir::Left, conn);
            }
            ShellAction::SnapRight => {
                self.wm.snap(SnapDir::Right, conn);
            }
            ShellAction::SnapUp => {
                self.wm.snap(SnapDir::Up, conn);
            }
            ShellAction::SnapDown => {
                self.wm.snap(SnapDir::Down, conn);
            }
            ShellAction::ResizeShrink => {
                self.wm.resize_split(false, conn);
            }
            ShellAction::ResizeGrow => {
                self.wm.resize_split(true, conn);
            }
            ShellAction::LockScreen => {
                self.lockscreen.lock();
                self.notifications.push_info("Session Locked");
            }
            ShellAction::Quit => {
                // ESC or Quit shortcut exits desktop shell
                std::process::exit(0);
            }
            ShellAction::None => {}
        }
    }

    pub fn handle_pointer_motion(&mut self, x: f64, y: f64, conn: &mut LgpConnection) {
        self.lockscreen.reset_idle();
        self.cursor_x = x;
        self.cursor_y = y;

        if self.lockscreen.locked { return; }

        self.taskbar.on_pointer_motion(x, y, self.screen_w);
        self.dock.on_pointer_motion(x, y, self.screen_w, self.screen_h);
        self.launcher.on_pointer_motion(x, y, self.screen_w, self.screen_h);
        self.wm.on_pointer_motion(x, y, conn);
    }

    pub fn handle_pointer_button(&mut self, x: f64, y: f64, button: u32, pressed: bool, conn: &mut LgpConnection) {
        self.lockscreen.reset_idle();
        if self.lockscreen.locked { return; }

        if pressed {
            // Check App Launcher overlay clicks
            if self.launcher.visible {
                if let Some(app) = self.launcher.on_click(x, y, self.screen_w, self.screen_h) {
                    let name = app.name;
                    let binary = app.binary;
                    self.launch_app_by_binary(name, binary);
                } else if !self.clicked_inside_launcher(x, y) {
                    self.launcher.visible = false;
                }
                return;
            }

            // Check Taskbar clicks
            if let Some(action) = self.taskbar.on_click(x, y, self.screen_w) {
                match action {
                    TaskbarAction::SwitchWorkspace(idx) => {
                        self.wm.switch_workspace(idx, conn);
                        self.notifications.push_info(format!("Workspace {}", idx + 1));
                    }
                    TaskbarAction::ToggleLauncher => {
                        self.launcher.visible = !self.launcher.visible;
                        if self.launcher.visible {
                            self.launcher.reset();
                        }
                    }
                }
                return;
            }

            // Check Dock clicks
            if let Some(idx) = self.dock.on_click(x, y, self.screen_w, self.screen_h) {
                if idx < self.dock.apps.len() {
                    self.launch_app(idx);
                }
                return;
            }

            // Check if user clicked on window decoration close/min buttons
            if let Some((sid, close_clicked)) = self.check_decorations_click(x, y) {
                if close_clicked {
                    let _ = lgp::wm::WmState::set_state(conn, sid, LGP_WM_STATE_HIDDEN);
                    self.wm.on_surface_destroyed(sid, conn);
                    self.notifications.push_info("Window closed");
                } else {
                    // Minimize window
                    let _ = lgp::wm::WmState::set_state(conn, sid, LGP_WM_STATE_MINIMIZED);
                    self.wm.on_surface_destroyed(sid, conn);
                    self.notifications.push_info("Window minimized");
                }
                return;
            }
        }

        // Pass to window manager for floating drag/move
        self.wm.on_pointer_button(x, y, button, pressed, conn);
    }

    fn clicked_inside_launcher(&self, x: f64, y: f64) -> bool {
        let lx = (self.screen_w as i32 - 400) / 2;
        let ly = (self.screen_h as i32 - 300) / 2;
        (x as i32) >= lx && (x as i32) < lx + 400 && (y as i32) >= ly && (y as i32) < ly + 300
    }

    fn check_decorations_click(&self, x: f64, y: f64) -> Option<(u32, bool)> {
        // Return (surface_id, is_close_button)
        for win in self.wm.active_windows() {
            let tx = win.x;
            let ty = win.y - 24;
            let tw = win.w as i32;
            let th = 24i32;

            let xi = x as i32;
            let yi = y as i32;

            if xi >= tx && xi < tx + tw && yi >= ty && yi < ty + th {
                let close_x = tx + tw - 16 - 4;
                let btn_y = ty + (24 - 16) / 2;
                if xi >= close_x && xi < close_x + 16 && yi >= btn_y && yi < btn_y + 16 {
                    return Some((win.surface_id, true));
                }
                let min_x = close_x - 16 - 4;
                if xi >= min_x && xi < min_x + 16 && yi >= btn_y && yi < btn_y + 16 {
                    return Some((win.surface_id, false));
                }
            }
        }
        None
    }

    fn launch_app(&mut self, idx: usize) {
        let name = self.dock.apps[idx].name;
        if self.dock.launch(idx).is_some() {
            self.notifications.push_success(format!("Launching {}", name));
        } else {
            self.notifications.push_error(format!("Failed to launch {}", name));
        }
    }

    fn launch_app_by_binary(&mut self, name: &str, binary: &str) {
        if let Some(idx) = self.dock.apps.iter().position(|a| a.binary == binary) {
            self.launch_app(idx);
        } else {
            // Spontaneous fork/exec for unmatched apps
            let pid = unsafe {
                let pid = libc::fork();
                if pid == 0 {
                    for fd in 3..256 { libc::close(fd); }
                    let path = std::ffi::CString::new(binary).unwrap_or_default();
                    let args = [path.as_ptr(), std::ptr::null()];
                    libc::execv(path.as_ptr(), args.as_ptr());
                    libc::_exit(127);
                }
                pid
            };
            if pid > 0 {
                self.notifications.push_success(format!("Launching {}", name));
            } else {
                self.notifications.push_error(format!("Failed to launch {}", name));
            }
        }
    }

    pub fn render_overlay(&self, pixels: &mut [u8], stride: u32) {
        // Clear overlay buffer to transparent alpha
        let byte_len = (stride as usize) * (self.screen_h as usize);
        pixels[..byte_len].fill(0);

        if self.lockscreen.locked {
            self.lockscreen.render(pixels, stride, self.screen_w, self.screen_h);
            return;
        }

        // Draw window decorations (borders, title bars) for active windows
        self.render_window_decorations(pixels, stride);

        // Draw topbar panel
        self.taskbar.render(pixels, stride, self.screen_w, self.screen_h, &self.wm, self.widgets.island.clock);

        // Draw bottom app dock
        self.dock.render(pixels, stride, self.screen_w, self.screen_h);

        // Draw desktop widgets (center stats/system info)
        self.widgets.render(pixels, stride, self.screen_w, self.screen_h);

        // Draw notifications toast queue
        self.notifications.render(pixels, stride, self.screen_w, self.screen_h);

        // Draw App Launcher search popup
        self.launcher.render(pixels, stride, self.screen_w, self.screen_h);
    }

    fn render_window_decorations(&self, pixels: &mut [u8], stride: u32) {
        for win in self.wm.active_windows() {
            let tx = win.x;
            let ty = win.y - 24;
            let tw = win.w as i32;
            let th = 24i32;

            let is_focused = self.wm.focused_surface == Some(win.surface_id);

            // Acrylic bar background
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                tx, ty, tw, th, 0xE60F1018);

            // Left side pink focused tag bar
            if is_focused {
                fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                    tx, ty, 4, th, 0xFFE03E8A);
            }

            // Bottom titlebar divider
            let div_color = if is_focused { 0xFFE03E8A } else { 0xFF5B3D78 };
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                tx, ty + th - 1, tw, 1, div_color);

            // Border outline around client window
            let outline_color = if is_focused { 0xFFE03E8A } else { 0xFF4A3A5A };
            // Left border
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                tx, win.y, 1, win.h as i32, outline_color);
            // Right border
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                tx + tw - 1, win.y, 1, win.h as i32, outline_color);
            // Bottom border
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                tx, win.y + win.h as i32 - 1, tw, 1, outline_color);

            // Window title text
            draw_text_on_slice(pixels, stride, self.screen_w, tx + 12, ty + 4, &win.title, 0xFFC8C8E0);

            // Close button [x]
            let btn_y = ty + (24 - 16) / 2;
            let close_x = tx + tw - 16 - 4;
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                close_x, btn_y, 16, 16, 0xFFFF4060);
            draw_text_on_slice(pixels, stride, self.screen_w, close_x + 4, btn_y + 1, "x", 0xFFFFFFFF);

            // Minimize button [-]
            let min_x = close_x - 16 - 4;
            fill_rect_on_slice(pixels, stride, self.screen_w, self.screen_h,
                min_x, btn_y, 16, 16, 0xFFF0A820);
            draw_text_on_slice(pixels, stride, self.screen_w, min_x + 5, btn_y + 1, "-", 0xFFFFFFFF);
        }
    }
}
