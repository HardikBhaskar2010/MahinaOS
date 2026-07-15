mod engine;
mod screens;

use lgp::client::LgpClient;
use lunagui::window::Window;
use screens::{Screen, WizardState};

fn main() {
    let mut client = LgpClient::connect("/tmp/lgp.sock").expect("Failed to connect to LGP Compositor");
    let window_id = client.create_window(800, 600).expect("Failed to create window");
    
    let mut window = Window::new(window_id, 800, 600, &mut client);
    
    let mut state = WizardState::Welcome;
    
    let mut welcome = screens::welcome::WelcomeScreen::new();
    let mut disk_select = screens::disk_select::DiskSelectScreen::new();
    let mut user_info = screens::user_info::UserInfoScreen::new();
    let mut progress = screens::progress::ProgressScreen::new();
    
    loop {
        // Handle input events
        while let Some(event) = client.poll_event() {
            match event {
                lgp::protocol::Event::MouseClick { window_id: wid, x, y, button: _ } => {
                    if wid == window_id {
                        let next_state = match state {
                            WizardState::Welcome => welcome.on_click(x as i32, y as i32),
                            WizardState::DiskSelect => disk_select.on_click(x as i32, y as i32),
                            WizardState::UserInfo => user_info.on_click(x as i32, y as i32),
                            WizardState::Progress => progress.on_click(x as i32, y as i32),
                            WizardState::Done => {
                                // Exit application on Done screen click
                                return;
                            }
                        };
                        
                        if let Some(ns) = next_state {
                            state = ns;
                            
                            // Handle state transitions
                            if state == WizardState::Progress {
                                let disk = disk_select.get_selected_disk().unwrap_or_else(|| "/dev/sda".to_string());
                                let username = user_info.username_field.text.clone();
                                let pass = user_info.password_field.text.clone();
                                let host = user_info.hostname_field.text.clone();
                                progress.start_installation(disk, username, pass, host);
                            }
                        }
                    }
                }
                lgp::protocol::Event::KeyPress { window_id: wid, keycode } => {
                    if wid == window_id {
                        match state {
                            WizardState::Welcome => welcome.on_key(keycode, true),
                            WizardState::DiskSelect => disk_select.on_key(keycode, true),
                            WizardState::UserInfo => user_info.on_key(keycode, true),
                            WizardState::Progress => progress.on_key(keycode, true),
                            WizardState::Done => {}
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Render
        let pixels = window.get_pixels_mut();
        
        match state {
            WizardState::Welcome => welcome.render(pixels, 800, 800, 600),
            WizardState::DiskSelect => disk_select.render(pixels, 800, 800, 600),
            WizardState::UserInfo => user_info.render(pixels, 800, 800, 600),
            WizardState::Progress => progress.render(pixels, 800, 800, 600),
            WizardState::Done => {
                lunagui::canvas::fill_rect_on_slice(pixels, 800, 0, 0, 800, 600, 0xFF101423);
                lunagui::canvas::draw_text_on_slice(pixels, 800, 300, 250, "Installation Finished. Reboot Now.", 0xFFFFFFFF);
            }
        }
        
        window.present(&mut client).expect("Failed to present window");
        
        std::thread::sleep(std::time::Duration::from_millis(16)); // roughly 60fps
    }
}
