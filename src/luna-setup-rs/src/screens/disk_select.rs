use super::{Screen, WizardState};
use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use lunagui::button::Button;
use lunagui::widget::Widget;
use std::fs;

pub struct DiskSelectScreen {
    next_btn: Button,
    back_btn: Button,
    disks: Vec<String>,
    selected_idx: Option<usize>,
}

impl DiskSelectScreen {
    pub fn new() -> Self {
        let mut disks = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/block") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("sd") || name.starts_with("nvme") || name.starts_with("vd") {
                    disks.push(format!("/dev/{}", name));
                }
            }
        }
        if disks.is_empty() {
            disks.push("/dev/sda".to_string()); // Fallback for testing
        }

        Self {
            next_btn: Button::new(0, 0, 150, 40, "Next", 0xFF3EE09A),
            back_btn: Button::new(0, 0, 150, 40, "Back", 0xFF6B7FD4),
            disks,
            selected_idx: None,
        }
    }

    pub fn get_selected_disk(&self) -> Option<String> {
        self.selected_idx.map(|i| self.disks[i].clone())
    }
}

impl Screen for DiskSelectScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        fill_rect_on_slice(pixels, stride, 0, 0, w, h, 0xFF101423); // Background
        draw_text_on_slice(pixels, stride, (w as i32 / 2) - 150, 80, "Select Target Disk", 0xFFFFFFFF);
        
        let mut y = 150;
        for (i, disk) in self.disks.iter().enumerate() {
            let color = if Some(i) == self.selected_idx { 0xFFE03E8A } else { 0xFF444455 };
            fill_rect_on_slice(pixels, stride, (w as i32 / 2) - 200, y, 400, 40, color);
            draw_text_on_slice(pixels, stride, (w as i32 / 2) - 180, y + 10, disk, 0xFFFFFFFF);
            y += 50;
        }

        self.back_btn.set_pos((w as i32 / 2) - 160, (h as i32) - 100);
        self.back_btn.render(pixels, stride, w, h);
        
        self.next_btn.set_pos((w as i32 / 2) + 10, (h as i32) - 100);
        self.next_btn.render(pixels, stride, w, h);
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        let w = 800; // Assuming 800x600 for now, we should track real size
        
        let mut list_y = 150;
        for i in 0..self.disks.len() {
            let bx = (w as i32 / 2) - 200;
            if x >= bx && x < bx + 400 && y >= list_y && y < list_y + 40 {
                self.selected_idx = Some(i);
                return None;
            }
            list_y += 50;
        }

        if self.back_btn.hit_test(x, y) {
            return Some(WizardState::Welcome);
        }
        if self.next_btn.hit_test(x, y) && self.selected_idx.is_some() {
            return Some(WizardState::UserInfo);
        }
        None
    }

    fn on_key(&mut self, _key: u32, _pressed: bool) {}
}
