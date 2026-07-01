/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * launcher.rs — Searchable App Launcher overlay.
 * Triggered by Super+Space or clicking the top-left logo.
 */

use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};

const LAUNCHER_W: i32 = 400;
const LAUNCHER_H: i32 = 300;
const ITEM_H: i32 = 32;
const BG: u32 = 0xEA0B0C16;
const BG_ITEM_HOVER: u32 = 0xFF242A44;
const ACCENT: u32 = 0xFFE03E8A;
const TEXT: u32 = 0xFFEEEEF4;
const TEXT_MUTED: u32 = 0xFF8888AA;

#[derive(Debug, Clone, Copy)]
pub struct LauncherItem {
    pub name: &'static str,
    pub desc: &'static str,
    pub binary: &'static str,
    pub icon_color: u32,
}

pub struct Launcher {
    pub visible: bool,
    pub items: Vec<LauncherItem>,
    pub search_query: String,
    pub selected_idx: usize,
    hover_idx: Option<usize>,
}

impl Launcher {
    pub fn new() -> Self {
        Self {
            visible: false,
            items: vec![
                LauncherItem { name: "Terminal",  desc: "Mahina System Console",  binary: "/usr/bin/luna-terminal",  icon_color: 0xFF3EE09A },
                LauncherItem { name: "Files",     desc: "File Navigator",        binary: "/usr/bin/files-rs",      icon_color: 0xFF6B7FD4 },
                LauncherItem { name: "Settings",  desc: "System Configuration",   binary: "/usr/bin/settings-rs",   icon_color: 0xFFE03E8A },
                LauncherItem { name: "Calculator",desc: "Simple Calculator",      binary: "/usr/bin/calc-rs",       icon_color: 0xFFE09A3E },
                LauncherItem { name: "Text Editor",desc: "Write and view text",   binary: "/usr/bin/text-rs",       icon_color: 0xFF9A3EE0 },
                LauncherItem { name: "Tasks",     desc: "Process Manager",        binary: "/usr/bin/tasks-rs",      icon_color: 0xFF3EE0C0 },
                LauncherItem { name: "About",     desc: "About MahinaOS",         binary: "/usr/bin/about-rs",       icon_color: 0xFF9898AA },
            ],
            search_query: String::new(),
            selected_idx: 0,
            hover_idx: None,
        }
    }

    pub fn reset(&mut self) {
        self.search_query.clear();
        self.selected_idx = 0;
        self.hover_idx = None;
    }

    pub fn filtered_items(&self) -> Vec<LauncherItem> {
        if self.search_query.is_empty() {
            return self.items.clone();
        }
        let query = self.search_query.to_lowercase();
        self.items.iter()
            .filter(|item| item.name.to_lowercase().contains(&query) || item.desc.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    pub fn on_pointer_motion(&mut self, x: f64, y: f64, sw: u32, sh: u32) {
        if !self.visible { return; }
        let lx = (sw as i32 - LAUNCHER_W) / 2;
        let ly = (sh as i32 - LAUNCHER_H) / 2;

        let xi = x as i32;
        let yi = y as i32;

        let list_y = ly + 44;
        let list_h = LAUNCHER_H - 52;

        if xi >= lx && xi < lx + LAUNCHER_W && yi >= list_y && yi < list_y + list_h {
            let relative_y = yi - list_y;
            let idx = (relative_y / ITEM_H) as usize;
            let count = self.filtered_items().len();
            if idx < count {
                self.hover_idx = Some(idx);
                return;
            }
        }
        self.hover_idx = None;
    }

    pub fn on_click(&mut self, x: f64, y: f64, sw: u32, sh: u32) -> Option<LauncherItem> {
        if !self.visible { return None; }
        self.on_pointer_motion(x, y, sw, sh);
        if let Some(h_idx) = self.hover_idx {
            let filtered = self.filtered_items();
            if h_idx < filtered.len() {
                self.visible = false;
                let item = filtered[h_idx];
                self.reset();
                return Some(item);
            }
        }
        None
    }

    pub fn on_key(&mut self, key: u32, pressed: bool) -> Option<LauncherItem> {
        if !self.visible || !pressed { return None; }

        match key {
            // Enter
            28 => {
                let filtered = self.filtered_items();
                if self.selected_idx < filtered.len() {
                    self.visible = false;
                    let item = filtered[self.selected_idx];
                    self.reset();
                    return Some(item);
                }
            }
            // Up Arrow
            103 => {
                let count = self.filtered_items().len();
                if count > 0 {
                    if self.selected_idx == 0 {
                        self.selected_idx = count - 1;
                    } else {
                        self.selected_idx -= 1;
                    }
                }
            }
            // Down Arrow
            108 => {
                let count = self.filtered_items().len();
                if count > 0 {
                    self.selected_idx = (self.selected_idx + 1) % count;
                }
            }
            // ESC
            1 => {
                self.visible = false;
                self.reset();
            }
            // Backspace
            14 => {
                self.search_query.pop();
                self.selected_idx = 0;
            }
            // Letter or space keys (evdev map)
            k => {
                if let Some(ch) = scancode_to_char(k) {
                    if self.search_query.len() < 32 {
                        self.search_query.push(ch);
                        self.selected_idx = 0;
                    }
                }
            }
        }
        None
    }

    pub fn render(&self, pixels: &mut [u8], stride: u32, sw: u32, sh: u32) {
        if !self.visible { return; }

        let lx = (sw as i32 - LAUNCHER_W) / 2;
        let ly = (sh as i32 - LAUNCHER_H) / 2;

        // Semitransparent dim backplate over the entire desktop
        fill_rect_on_slice(pixels, stride, sw, sh, 0, 0, sw as i32, sh as i32, 0x70000000);

        // Launcher panel background
        fill_rect_on_slice(pixels, stride, sw, sh, lx, ly, LAUNCHER_W, LAUNCHER_H, BG);

        // Borders
        fill_rect_on_slice(pixels, stride, sw, sh, lx, ly, LAUNCHER_W, 1, ACCENT);
        fill_rect_on_slice(pixels, stride, sw, sh, lx, ly + LAUNCHER_H - 1, LAUNCHER_W, 1, ACCENT);
        fill_rect_on_slice(pixels, stride, sw, sh, lx, ly, 1, LAUNCHER_H, ACCENT);
        fill_rect_on_slice(pixels, stride, sw, sh, lx + LAUNCHER_W - 1, ly, 1, LAUNCHER_H, ACCENT);

        // Search Bar container
        let sb_y = ly + 10;
        fill_rect_on_slice(pixels, stride, sw, sh, lx + 10, sb_y, LAUNCHER_W - 20, 24, 0xFF161626);
        fill_rect_on_slice(pixels, stride, sw, sh, lx + 10, sb_y, LAUNCHER_W - 20, 1, 0xFF303050);
        fill_rect_on_slice(pixels, stride, sw, sh, lx + 10, sb_y + 23, LAUNCHER_W - 20, 1, 0xFF303050);
        fill_rect_on_slice(pixels, stride, sw, sh, lx + 10, sb_y, 1, 24, 0xFF303050);
        fill_rect_on_slice(pixels, stride, sw, sh, lx + LAUNCHER_W - 11, sb_y, 1, 24, 0xFF303050);

        // Search Query / Placeholder
        if self.search_query.is_empty() {
            draw_text_on_slice(pixels, stride, sw, lx + 18, sb_y + 4, "Type to search...", TEXT_MUTED);
        } else {
            let cursor = if (self.search_query.len() % 2) == 0 { "_" } else { "" };
            let display_str = format!("{}{}", self.search_query, cursor);
            draw_text_on_slice(pixels, stride, sw, lx + 18, sb_y + 4, &display_str, TEXT);
        }

        // Render filtered items
        let filtered = self.filtered_items();
        let list_y = ly + 44;
        let max_visible = ((LAUNCHER_H - 54) / ITEM_H) as usize;

        for i in 0..filtered.len().min(max_visible) {
            let item = filtered[i];
            let iy = list_y + i as i32 * ITEM_H;

            let is_selected = i == self.selected_idx;
            let is_hover = self.hover_idx == Some(i);

            let bg = if is_selected {
                BG_ITEM_HOVER
            } else if is_hover {
                0x30FFFFFF
            } else {
                0x00000000
            };

            if bg != 0 {
                fill_rect_on_slice(pixels, stride, sw, sh, lx + 6, iy, LAUNCHER_W - 12, ITEM_H - 2, bg);
            }

            // Left colored tag indicator
            fill_rect_on_slice(pixels, stride, sw, sh, lx + 10, iy + 4, 4, ITEM_H - 10, item.icon_color);

            // Item Name
            draw_text_on_slice(pixels, stride, sw, lx + 22, iy + 8, item.name, TEXT);

            // Item Description (muted)
            let desc_x = lx + 140;
            draw_text_on_slice(pixels, stride, sw, desc_x, iy + 8, item.desc, TEXT_MUTED);
        }
    }
}

fn scancode_to_char(code: u32) -> Option<char> {
    const MAP: &[(u32, char)] = &[
        (16,'q'),(17,'w'),(18,'e'),(19,'r'),(20,'t'),(21,'y'),(22,'u'),(23,'i'),
        (24,'o'),(25,'p'),(30,'a'),(31,'s'),(32,'d'),(33,'f'),(34,'g'),(35,'h'),
        (36,'j'),(37,'k'),(38,'l'),(44,'z'),(45,'x'),(46,'c'),(47,'v'),(48,'b'),
        (49,'n'),(50,'m'),(57,' '),
    ];
    MAP.iter().find(|&&(c, _)| c == code).map(|&(_, ch)| ch)
}
