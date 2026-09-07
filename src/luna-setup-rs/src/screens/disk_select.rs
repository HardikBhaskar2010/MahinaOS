use super::{render_wizard_chrome, Screen, WizardState, COLOR_ACCENT, COLOR_BORDER, COLOR_BORDER_ACCENT, COLOR_CARD_INNER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SEC, COLOR_WARN};
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};
use lunagui::Button;
use lunagui::Widget;
use std::fs;

pub struct DiskItem {
    pub path: String,
    pub name: String,
    pub size_gb: u64,
}

pub struct DiskSelectScreen {
    next_btn: Button,
    back_btn: Button,
    disks: Vec<DiskItem>,
    selected_idx: Option<usize>,
}

impl DiskSelectScreen {
    pub fn new() -> Self {
        let mut disks = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/block") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("sd") || name.starts_with("nvme") || name.starts_with("vd") {
                    let mut size_gb = 0;
                    let size_path = format!("/sys/block/{}/size", name);
                    if let Ok(size_str) = fs::read_to_string(&size_path) {
                        if let Ok(sectors) = size_str.trim().parse::<u64>() {
                            size_gb = (sectors * 512) / (1024 * 1024 * 1024);
                        }
                    }
                    disks.push(DiskItem {
                        path: format!("/dev/{}", name),
                        name,
                        size_gb,
                    });
                }
            }
        }
        if disks.is_empty() {
            disks.push(DiskItem {
                path: "/dev/vda".to_string(),
                name: "vda (QEMU VirtIO)".to_string(),
                size_gb: 20,
            });
        }

        Self {
            next_btn: Button::accent(0, 0, 160, 36, "Configure User ->"),
            back_btn: Button::new(0, 0, 120, 36, "<- Back"),
            disks,
            selected_idx: Some(0), // Default to first disk
        }
    }

    pub fn get_selected_disk(&self) -> Option<String> {
        self.selected_idx.map(|i| self.disks[i].path.clone())
    }
}

impl Screen for DiskSelectScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        render_wizard_chrome(pixels, stride, w, h, 2, "Select Target Storage Device");

        let card_w = 680i32;
        let card_h = 500i32;
        let card_x = (w as i32 - card_w) / 2;
        let card_y = (h as i32 - card_h) / 2;

        let content_x = card_x + 24;
        let mut y = card_y + 96;

        draw_text_on_slice(pixels, stride, w, content_x, y, "Choose the drive where MahinaOS will be installed:", COLOR_TEXT_SEC);
        y += 24;

        // Render disk cards
        for (i, d) in self.disks.iter().enumerate() {
            let is_sel = Some(i) == self.selected_idx;
            let disk_card_w = card_w - 48;
            let disk_card_h = 44i32;

            let border_col = if is_sel { COLOR_BORDER_ACCENT } else { COLOR_BORDER };
            let bg_col = if is_sel { COLOR_CARD_INNER } else { COLOR_CARD_INNER };

            fill_rect_on_slice(pixels, stride, w, h, content_x - 1, y - 1, disk_card_w + 2, disk_card_h + 2, border_col);
            fill_rect_on_slice(pixels, stride, w, h, content_x, y, disk_card_w, disk_card_h, bg_col);

            let marker = if is_sel { "(O)" } else { "( )" };
            let marker_col = if is_sel { COLOR_ACCENT } else { COLOR_TEXT_MUTED };
            draw_text_on_slice(pixels, stride, w, content_x + 16, y + 14, marker, marker_col);

            let label = format!("{} — {} GB capacity [{}]", d.path, d.size_gb, d.name);
            let text_col = if is_sel { COLOR_TEXT_PRIMARY } else { COLOR_TEXT_SEC };
            draw_text_on_slice(pixels, stride, w, content_x + 50, y + 14, &label, text_col);

            y += disk_card_h + 12;
        }

        y = card_y + 300;
        // Warning box
        let warn_w = card_w - 48;
        let warn_h = 80;
        fill_rect_on_slice(pixels, stride, w, h, content_x - 1, y - 1, warn_w + 2, warn_h + 2, COLOR_BORDER);
        fill_rect_on_slice(pixels, stride, w, h, content_x, y, warn_w, warn_h, COLOR_CARD_INNER);

        draw_text_on_slice(pixels, stride, w, content_x + 16, y + 12, "CAUTION: Automated Disk Formatting", COLOR_WARN);
        draw_text_on_slice(pixels, stride, w, content_x + 16, y + 32, "The selected drive will be formatted with GPT and Btrfs subvolumes:", COLOR_TEXT_MUTED);
        draw_text_on_slice(pixels, stride, w, content_x + 16, y + 50, "@ (Root), @home (User Data), @snapshots, @generations (Rollback).", COLOR_TEXT_MUTED);

        // Buttons
        self.back_btn.set_pos(content_x, card_y + card_h - 24 - 36);
        self.back_btn.render(pixels, stride, w, h);

        self.next_btn.set_pos(card_x + card_w - 24 - 160, card_y + card_h - 24 - 36);
        self.next_btn.render(pixels, stride, w, h);
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        let card_w = 680i32;
        let card_h = 500i32;
        let card_x = (800i32 - card_w) / 2;
        let card_y = (600i32 - card_h) / 2;
        let content_x = card_x + 24;

        let mut list_y = card_y + 120;
        for i in 0..self.disks.len() {
            let bx = content_x;
            let bw = card_w - 48;
            if x >= bx && x < bx + bw && y >= list_y && y < list_y + 44 {
                self.selected_idx = Some(i);
                return None;
            }
            list_y += 56;
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
