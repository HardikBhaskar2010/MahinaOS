mod engine;
mod screens;

use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};
use screens::{render_wizard_chrome, Screen, WizardState, COLOR_CARD_INNER, COLOR_BORDER, COLOR_SUCCESS, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SEC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER)?;

    let width = 800u32;
    let height = 600u32;

    let mut surface = LgpSurface::create(
        &mut conn,
        LGP_SURFACE_CANVAS_SURFACE,
        0, 0,
        width, height,
        LGP_LAYER_APPLICATION,
        true,
    )?;

    conn.set_nonblocking(true)?;

    let mut state = WizardState::Welcome;

    let mut welcome = screens::welcome::WelcomeScreen::new();
    let mut disk_select = screens::disk_select::DiskSelectScreen::new();
    let mut user_info = screens::user_info::UserInfoScreen::new();
    let mut progress = screens::progress::ProgressScreen::new();

    let mut cursor_x: i32 = 0;
    let mut cursor_y: i32 = 0;

    loop {
        // 1. Render active screen
        {
            let pixels = surface.pixels();
            let stride = width * 4;

            match state {
                WizardState::Welcome => welcome.render(pixels, stride, width, height),
                WizardState::DiskSelect => disk_select.render(pixels, stride, width, height),
                WizardState::UserInfo => user_info.render(pixels, stride, width, height),
                WizardState::Progress => progress.render(pixels, stride, width, height),
                WizardState::Done => {
                    render_wizard_chrome(pixels, stride, width, height, 4, "Installation Complete");

                    let card_w = 680i32;
                    let card_h = 500i32;
                    let card_x = (width as i32 - card_w) / 2;
                    let card_y = (height as i32 - card_h) / 2;

                    let cx = card_x + 36;
                    let y = card_y + 120;

                    let box_w = card_w - 72;
                    let box_h = 160;
                    fill_rect_on_slice(pixels, stride, width, height, cx - 1, y - 1, box_w + 2, box_h + 2, COLOR_BORDER);
                    fill_rect_on_slice(pixels, stride, width, height, cx, y, box_w, box_h, COLOR_CARD_INNER);

                    draw_text_on_slice(pixels, stride, width, cx + 24, y + 24, "MahinaOS has been successfully installed!", COLOR_SUCCESS);
                    draw_text_on_slice(pixels, stride, width, cx + 24, y + 54, "The Btrfs root filesystem, subvolume hierarchy, and Limine", COLOR_TEXT_PRIMARY);
                    draw_text_on_slice(pixels, stride, width, cx + 24, y + 74, "bootloader have been provisioned on your target drive.", COLOR_TEXT_PRIMARY);
                    draw_text_on_slice(pixels, stride, width, cx + 24, y + 104, "Please remove installation media and reboot to enter MahinaOS.", COLOR_TEXT_SEC);
                    draw_text_on_slice(pixels, stride, width, cx + 24, y + 124, "Press [ENTER] or click anywhere to exit installer.", COLOR_TEXT_MUTED);
                }
            }
        }

        surface.commit(&mut conn)?;

        // 2. Poll IPC socket events
        let mut pfd = pollfd {
            fd: conn.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };
        let ret = unsafe { poll(&mut pfd, 1, 16) }; // ~60fps poll interval
        if ret < 0 { continue; }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
                    t if t == LgpMessageType::PointerMotion as u16 => {
                        if let Some(ev) = parse_pointer_motion(&msg.payload) {
                            cursor_x = ev.x as i32;
                            cursor_y = ev.y as i32;
                        }
                    }
                    t if t == LgpMessageType::PointerButton as u16 => {
                        if let Some(ev) = parse_pointer_button(&msg.payload) {
                            if ev.pressed {
                                let next_state = match state {
                                    WizardState::Welcome => welcome.on_click(cursor_x, cursor_y),
                                    WizardState::DiskSelect => disk_select.on_click(cursor_x, cursor_y),
                                    WizardState::UserInfo => user_info.on_click(cursor_x, cursor_y),
                                    WizardState::Progress => progress.on_click(cursor_x, cursor_y),
                                    WizardState::Done => {
                                        return Ok(());
                                    }
                                };

                                if let Some(ns) = next_state {
                                    state = ns;

                                    if state == WizardState::Progress {
                                        let disk = disk_select.get_selected_disk().unwrap_or_else(|| "/dev/vda".to_string());
                                        let username = user_info.username_field.text.clone();
                                        let pass = user_info.password_field.text.clone();
                                        let host = user_info.hostname_field.text.clone();
                                        progress.start_installation(disk, username, pass, host);
                                    }
                                }
                            }
                        }
                    }
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.pressed && ev.key == 0x70029 { // Escape
                                return Ok(());
                            }
                            if state == WizardState::Done && ev.pressed && (ev.key == 0x70028 || ev.key == 0x28) {
                                return Ok(());
                            }
                            match state {
                                WizardState::Welcome => welcome.on_key(ev.key, ev.pressed),
                                WizardState::DiskSelect => disk_select.on_key(ev.key, ev.pressed),
                                WizardState::UserInfo => user_info.on_key(ev.key, ev.pressed),
                                WizardState::Progress => progress.on_key(ev.key, ev.pressed),
                                WizardState::Done => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
