use super::{render_wizard_chrome, Screen, WizardState, COLOR_ACCENT, COLOR_BORDER_ACCENT, COLOR_CARD_INNER, COLOR_ERROR, COLOR_SUCCESS, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SEC};
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};
use lunagui::Button;
use lunagui::Widget;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ProgressScreen {
    pub status: Arc<Mutex<String>>,
    pub step_index: Arc<Mutex<usize>>,
    pub is_done: Arc<Mutex<bool>>,
    pub error: Arc<Mutex<Option<String>>>,
    finish_btn: Button,
    started: bool,
}

impl ProgressScreen {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new("Preparing installation environment...".to_string())),
            step_index: Arc::new(Mutex::new(0)),
            is_done: Arc::new(Mutex::new(false)),
            error: Arc::new(Mutex::new(None)),
            finish_btn: Button::accent(0, 0, 200, 36, "Reboot into MahinaOS"),
            started: false,
        }
    }

    pub fn start_installation(&mut self, disk: String, username: String, pass: String, host: String) {
        if self.started { return; }
        self.started = true;

        let status = self.status.clone();
        let step_index = self.step_index.clone();
        let is_done = self.is_done.clone();
        let error = self.error.clone();

        thread::spawn(move || {
            let set_step = |idx: usize, msg: &str| {
                if let Ok(mut si) = step_index.lock() { *si = idx; }
                if let Ok(mut s) = status.lock() { *s = msg.to_string(); }
            };

            // 1. Partitioning
            set_step(1, "Creating GPT partition table and EFI/Btrfs partitions...");
            if let Err(e) = crate::engine::disk::partition_disk(&disk) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("Partitioning failed: {:?}", e)); }
                return;
            }

            // 2. Formatting & Subvolumes
            set_step(2, "Formatting ESP (FAT32) and Root (Btrfs subvolumes: @, @home, @snapshots)...");
            if let Err(e) = crate::engine::disk::format_partitions(&disk) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("Formatting failed: {:?}", e)); }
                return;
            }

            // 3. Mounting target
            set_step(3, "Mounting Btrfs subvolumes and ESP at /mnt/mahina-target...");
            let target_mnt = "/mnt/mahina-target";
            if let Err(e) = crate::engine::disk::mount_target(&disk, target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("Mount failed: {:?}", e)); }
                return;
            }

            // 4. Copying Files
            set_step(4, "Deploying MahinaOS base system binaries, libraries, and assets...");
            if let Err(e) = crate::engine::system::copy_rootfs(target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("System copy failed: {:?}", e)); }
                return;
            }

            // 5. Configs & User Identity
            set_step(5, "Configuring /etc/fstab, user account, hostname, and permissions...");
            if let Err(e) = crate::engine::system::generate_fstab(&disk, target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("fstab failed: {:?}", e)); }
                return;
            }
            if let Err(e) = crate::engine::user::create_user(target_mnt, &username, &username, &pass) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("User creation failed: {:?}", e)); }
                return;
            }
            if let Err(e) = crate::engine::user::set_hostname(target_mnt, &host) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("Hostname failed: {:?}", e)); }
                return;
            }

            // 6. Limine Bootloader
            set_step(6, "Installing Limine UEFI bootloader and kernel artifacts...");
            if let Err(e) = crate::engine::system::install_bootloader(target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("Bootloader installation failed: {:?}", e)); }
                return;
            }

            // Done
            set_step(7, "Installation successfully completed! System is ready to boot.");
            if let Ok(mut d) = is_done.lock() { *d = true; }
        });
    }
}

impl Screen for ProgressScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        render_wizard_chrome(pixels, stride, w, h, 4, "System Installation in Progress");

        let card_w = 680i32;
        let card_h = 500i32;
        let card_x = (w as i32 - card_w) / 2;
        let card_y = (h as i32 - card_h) / 2;

        let content_x = card_x + 36;
        let mut y = card_y + 86;

        let curr_step = *self.step_index.lock().unwrap();
        let done = *self.is_done.lock().unwrap();
        let error_opt = self.error.lock().unwrap().clone();

        draw_text_on_slice(pixels, stride, w, content_x, y, "Applying system configuration to disk:", COLOR_TEXT_SEC);
        y += 24;

        // Step checklist items
        let step_names = [
            "1. GPT Disk Partitioning (ESP + Btrfs Root)",
            "2. Filesystem & Subvolumes (@, @home, @snapshots, @generations)",
            "3. Target Mount Structure (/mnt/mahina-target)",
            "4. Base System Deployment (Kernel, Core, Experience)",
            "5. User Identity & Fstab Generation",
            "6. Limine UEFI Bootloader Installation",
        ];

        for (i, name) in step_names.iter().enumerate() {
            let step_num = i + 1;
            let (indicator, color) = if done || curr_step > step_num {
                ("[OK]", COLOR_SUCCESS)
            } else if curr_step == step_num && error_opt.is_none() {
                ("[>>]", COLOR_ACCENT)
            } else if curr_step == step_num && error_opt.is_some() {
                ("[!!]", COLOR_ERROR)
            } else {
                ("[  ]", COLOR_TEXT_MUTED)
            };

            draw_text_on_slice(pixels, stride, w, content_x, y, indicator, color);
            draw_text_on_slice(pixels, stride, w, content_x + 40, y, name, if curr_step >= step_num { COLOR_TEXT_PRIMARY } else { COLOR_TEXT_MUTED });
            y += 22;
        }

        y += 12;
        // Progress status card
        let status_box_w = card_w - 72;
        let status_box_h = 70;
        let status_border = if error_opt.is_some() { COLOR_ERROR } else if done { COLOR_SUCCESS } else { COLOR_BORDER_ACCENT };
        fill_rect_on_slice(pixels, stride, w, h, content_x - 1, y - 1, status_box_w + 2, status_box_h + 2, status_border);
        fill_rect_on_slice(pixels, stride, w, h, content_x, y, status_box_w, status_box_h, COLOR_CARD_INNER);

        let status_msg = self.status.lock().unwrap().clone();
        draw_text_on_slice(pixels, stride, w, content_x + 12, y + 14, "Status:", COLOR_TEXT_MUTED);
        draw_text_on_slice(pixels, stride, w, content_x + 80, y + 14, &status_msg, if error_opt.is_some() { COLOR_ERROR } else { COLOR_SUCCESS });

        if let Some(ref err) = error_opt {
            draw_text_on_slice(pixels, stride, w, content_x + 12, y + 38, &format!("Fault: {}", &err[..std::cmp::min(err.len(), 65)]), COLOR_ERROR);
        } else if done {
            draw_text_on_slice(pixels, stride, w, content_x + 12, y + 38, "Ready to reboot into MahinaOS.", COLOR_TEXT_SEC);
        }

        // Action button
        if done || error_opt.is_some() {
            self.finish_btn.set_pos(card_x + card_w - 36 - 200, card_y + card_h - 24 - 36);
            self.finish_btn.render(pixels, stride, w, h);
        }
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        let done = *self.is_done.lock().unwrap();
        let has_err = self.error.lock().unwrap().is_some();

        if (done || has_err) && self.finish_btn.hit_test(x, y) {
            return Some(WizardState::Done);
        }
        None
    }

    fn on_key(&mut self, _key: u32, _pressed: bool) {}
}
