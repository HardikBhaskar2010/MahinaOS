/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * taskbar.rs — Top panel for MahinaOS Desktop Shell.
 * Shows: app menu trigger, workspace tabs, active window title, system tray, clock.
 */

use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use crate::wm::{WmManager, NUM_WORKSPACES};

pub const TASKBAR_H: i32 = 32;
const ACCENT: u32 = 0xFFE03E8A;
const TEXT_COLOR: u32 = 0xFFD8D8E8;
const TEXT_DIM: u32 = 0xFF888899;
const BG: u32 = 0xF0070710;
const WS_ACTIVE_BG: u32 = 0xFF1A1A30;
const WS_INACTIVE_BG: u32 = 0x00000000;

pub struct Taskbar {
    pub show_launcher: bool,
    hover_ws: Option<usize>,
}

impl Taskbar {
    pub fn new() -> Self {
        Self { show_launcher: false, hover_ws: None }
    }

    pub fn on_pointer_motion(&mut self, x: f64, _y: f64, sw: u32) {
        // Detect hover over workspace tabs
        let ws_start = 80i32;
        let ws_tab_w = 40i32;
        let xi = x as i32;
        if xi >= ws_start && (_y as i32) < TASKBAR_H {
            let idx = ((xi - ws_start) / ws_tab_w) as usize;
            if idx < NUM_WORKSPACES {
                self.hover_ws = Some(idx);
                return;
            }
        }
        self.hover_ws = None;
    }

    /// Handle a click on the taskbar. Returns Some(ws_index) if a workspace was clicked.
    pub fn on_click(&mut self, x: f64, y: f64, _sw: u32) -> Option<TaskbarAction> {
        if y as i32 > TASKBAR_H { return None; }

        let xi = x as i32;

        // App menu button (leftmost, "⌘ Mahina")
        if xi < 80 {
            self.show_launcher = !self.show_launcher;
            return Some(TaskbarAction::ToggleLauncher);
        }

        // Workspace tabs
        let ws_start = 80i32;
        let ws_tab_w = 40i32;
        if xi >= ws_start && xi < ws_start + ws_tab_w * NUM_WORKSPACES as i32 {
            let idx = ((xi - ws_start) / ws_tab_w) as usize;
            if idx < NUM_WORKSPACES {
                return Some(TaskbarAction::SwitchWorkspace(idx));
            }
        }

        None
    }

    pub fn render(&self, pixels: &mut [u8], stride: u32, sw: u32, sh: u32,
                  wm: &WmManager, clock: (u8, u8, u8)) {
        let w = sw as i32;
        let h = TASKBAR_H;

        // Background with slight gradient
        fill_rect_on_slice(pixels, stride, sw, sh, 0, 0, w, h, BG);

        // Bottom separator line (accent)
        fill_rect_on_slice(pixels, stride, sw, sh, 0, h - 1, w, 1, ACCENT);

        // ── App menu button ───────────────────────────────────────────────
        let logo = "MahinaOS";
        fill_rect_on_slice(pixels, stride, sw, sh, 0, 0, 80, h, 0x20E03E8A);
        draw_text_on_slice(pixels, stride, sw, 6, 8, logo, ACCENT);

        // Separator
        fill_rect_on_slice(pixels, stride, sw, sh, 80, 4, 1, h - 8, 0xFF222244);

        // ── Workspace tabs ────────────────────────────────────────────────
        let ws_start = 82i32;
        let ws_tab_w = 40i32;
        let ws_names = ["1", "2", "3", "4", "5", "6"];
        let active_ws = wm.active_workspace;

        for (i, name) in ws_names.iter().enumerate() {
            let tx = ws_start + i as i32 * ws_tab_w;
            let is_active = i == active_ws;
            let is_hover = self.hover_ws == Some(i);

            // Has windows?
            let has_windows = wm.windows.values().any(|w| w.workspace == i);

            let bg = if is_active { WS_ACTIVE_BG }
                     else if is_hover { 0x15FFFFFF }
                     else { WS_INACTIVE_BG };
            fill_rect_on_slice(pixels, stride, sw, sh, tx, 0, ws_tab_w - 1, h, bg);

            let text_color = if is_active { ACCENT } else { TEXT_DIM };
            let label_x = tx + (ws_tab_w - 8) / 2;
            draw_text_on_slice(pixels, stride, sw, label_x, 8, name, text_color);

            // Active indicator line at bottom
            if is_active {
                fill_rect_on_slice(pixels, stride, sw, sh, tx + 4, h - 3, ws_tab_w - 8, 2, ACCENT);
            }

            // Dot indicator for occupied workspaces
            if has_windows && !is_active {
                fill_rect_on_slice(pixels, stride, sw, sh, tx + ws_tab_w / 2 - 2, h - 5, 4, 2, 0xFF9898AA);
            }
        }

        // Separator after workspace tabs
        let ws_end = ws_start + ws_tab_w * NUM_WORKSPACES as i32;
        fill_rect_on_slice(pixels, stride, sw, sh, ws_end, 4, 1, h - 8, 0xFF222244);

        // ── Active window title ───────────────────────────────────────────
        if let Some(sid) = wm.focused_surface {
            if let Some(win) = wm.windows.get(&sid) {
                let title = &win.title;
                let max_title_w = (w - ws_end - 200) / 8;
                let display_title = if title.len() > max_title_w as usize {
                    &title[..max_title_w as usize]
                } else {
                    title.as_str()
                };
                let title_x = ws_end + 8;
                draw_text_on_slice(pixels, stride, sw, title_x, 8, display_title, TEXT_COLOR);
            }
        }

        // ── Right side: tray icons + clock ────────────────────────────────
        let clock_str = format!("{:02}:{:02}", clock.0, clock.1);
        let clock_x = w - clock_str.len() as i32 * 8 - 8;
        draw_text_on_slice(pixels, stride, sw, clock_x, 8, &clock_str, TEXT_COLOR);

        // Network icon placeholder (wifi bars)
        let net_x = clock_x - 24;
        for bar in 0..4i32 {
            let bar_h = (bar + 1) * 3;
            fill_rect_on_slice(pixels, stride, sw, sh,
                net_x + bar * 6, h - 4 - bar_h, 4, bar_h, 0xFF5566AA);
        }

        // Volume icon placeholder
        let vol_x = net_x - 20;
        draw_text_on_slice(pixels, stride, sw, vol_x, 8, "VOL", TEXT_DIM);
    }
}

pub enum TaskbarAction {
    SwitchWorkspace(usize),
    ToggleLauncher,
}
