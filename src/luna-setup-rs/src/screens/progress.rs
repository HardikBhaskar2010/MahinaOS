use super::{Screen, WizardState};
use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ProgressScreen {
    pub status: Arc<Mutex<String>>,
    pub is_done: Arc<Mutex<bool>>,
    pub error: Arc<Mutex<Option<String>>>,
    started: bool,
}

impl ProgressScreen {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new("Ready...".to_string())),
            is_done: Arc::new(Mutex::new(false)),
            error: Arc::new(Mutex::new(None)),
            started: false,
        }
    }

    pub fn start_installation(&mut self, disk: String, username: String, pass: String, host: String) {
        if self.started { return; }
        self.started = true;
        
        let status = self.status.clone();
        let is_done = self.is_done.clone();
        let error = self.error.clone();
        
        thread::spawn(move || {
            let update_status = |msg: &str| {
                if let Ok(mut s) = status.lock() {
                    *s = msg.to_string();
                }
            };
            
            // 1. Partitioning
            update_status("Partitioning disk...");
            if let Err(e) = crate::engine::disk::partition_disk(&disk) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            // 2. Formatting
            update_status("Formatting partitions...");
            if let Err(e) = crate::engine::disk::format_partitions(&disk) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            // 3. Mounting
            update_status("Mounting target...");
            let target_mnt = "/mnt/luna-target";
            if let Err(e) = crate::engine::disk::mount_target(&disk, target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            // 4. Copying Files
            update_status("Copying system files...");
            if let Err(e) = crate::engine::system::copy_rootfs(target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            // 5. Configs
            update_status("Generating fstab...");
            if let Err(e) = crate::engine::system::generate_fstab(&disk, target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            update_status("Configuring user...");
            if let Err(e) = crate::engine::user::create_user(target_mnt, &username, &username, &pass) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            if let Err(e) = crate::engine::user::set_hostname(target_mnt, &host) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            // 6. Bootloader
            update_status("Installing bootloader...");
            if let Err(e) = crate::engine::system::install_bootloader(target_mnt) {
                if let Ok(mut err) = error.lock() { *err = Some(format!("{:?}", e)); }
                return;
            }
            
            // Done
            update_status("Installation complete!");
            if let Ok(mut d) = is_done.lock() { *d = true; }
        });
    }
}

impl Screen for ProgressScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        fill_rect_on_slice(pixels, stride, 0, 0, w, h, 0xFF101423); // Background
        draw_text_on_slice(pixels, stride, (w as i32 / 2) - 150, 150, "Installing LunaOS...", 0xFFFFFFFF);
        
        let cx = (w as i32 / 2) - 200;
        
        let status = self.status.lock().unwrap().clone();
        draw_text_on_slice(pixels, stride, cx, 250, &status, 0xFF3EE09A);
        
        if let Some(err) = self.error.lock().unwrap().as_ref() {
            draw_text_on_slice(pixels, stride, cx, 300, "Error:", 0xFFFF4444);
            // Quick and dirty line wrap for error
            draw_text_on_slice(pixels, stride, cx, 330, &err[..std::cmp::min(err.len(), 50)], 0xFFFF4444);
        }
    }

    fn on_click(&mut self, _x: i32, _y: i32) -> Option<WizardState> {
        let done = *self.is_done.lock().unwrap();
        let has_err = self.error.lock().unwrap().is_some();
        if done || has_err {
            return Some(WizardState::Done);
        }
        None
    }

    fn on_key(&mut self, _key: u32, _pressed: bool) {}
}
