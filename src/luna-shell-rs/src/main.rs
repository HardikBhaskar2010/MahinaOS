/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * main.rs — Entry point for the MahinaOS Desktop Shell.
 */

mod keybinds;
mod notifications;
mod wallpaper;
mod widgets;
mod wm;
mod lockscreen;
mod taskbar;
mod dock;
mod launcher;
mod shell;

use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use crate::shell::ShellState;

fn grab_wm_keys(conn: &mut LgpConnection) -> std::io::Result<()> {
    use crate::keybinds::keys::*;
    use lgp::protocol::{LGP_MOD_SUPER, LGP_MOD_SHIFT, LGP_MOD_CTRL, LGP_MOD_ALT};

    // Grab all shortcut combinations we support
    let keys_list = [
        KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6,
        KEY_Q, KEY_W, KEY_E, KEY_T, KEY_F, KEY_J, KEY_K, KEY_L, KEY_V,
        KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT, KEY_TAB, KEY_ENTER, KEY_SPACE,
        KEY_ESC, KEY_BACKSPACE, KEY_0, KEY_9, KEY_8, KEY_7
    ];

    let modifiers = [
        0,
        LGP_MOD_SUPER,
        LGP_MOD_SUPER | LGP_MOD_SHIFT,
        LGP_MOD_SUPER | LGP_MOD_CTRL,
        LGP_MOD_ALT,
    ];

    for &key in &keys_list {
        for &mods in &modifiers {
            let _ = lgp::wm::WmState::grab_key(conn, key, mods);
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let caps = LGP_CAP_LAYER_SHELL
        | LGP_CAP_CANVAS_SURFACE
        | LGP_CAP_KEYBOARD
        | LGP_CAP_POINTER
        | LGP_CAP_WINDOW_MANAGER;

    let mut conn = LgpConnection::connect(caps)?;
    unsafe {
        let fd = conn.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
        }
    }
    let sw = conn.output_width;
    let sh = conn.output_height;

    // Grab shortcuts globally
    grab_wm_keys(&mut conn)?;

    // Create background wallpaper surface (layer 0, opaque XRGB)
    let mut wallpaper = LgpSurface::create(
        &mut conn,
        LGP_SURFACE_CANVAS_SURFACE,
        0, 0,
        sw, sh,
        LGP_LAYER_WALLPAPER,
        false,
    )?;

    // Create desktop UI overlay surface (layer 200, transparent ARGB)
    let mut overlay = LgpSurface::create(
        &mut conn,
        LGP_SURFACE_SHELL_SURFACE,
        0, 0,
        sw, sh,
        LGP_LAYER_SHELL,
        true,
    )?;

    let mut state = ShellState::new(sw, sh);

    // Initial lock screen keyboard focus grab
    let _ = lgp::wm::WmState::set_focus(&mut conn, overlay.id);

    // Event loop polling variables
    let mut pfd = pollfd {
        fd: conn.as_raw_fd(),
        events: POLLIN,
        revents: 0,
    };

    // Shadow buffers to avoid flickering from direct LgpSurface writes
    let mut wp_shadow = vec![0u8; (sw * sh * 4) as usize];
    let mut ov_shadow = vec![0u8; (sw * sh * 4) as usize];

    let mut dirty = true;

    loop {
        // 1. Tick state (animations, stats updates, clock, child reaping)
        state.tick(&mut conn);

        // 2. Render and commit wallpaper & UI overlay only when dirty or during active animations
        if dirty {
            state.wallpaper.render(&mut wp_shadow, sw * 4);
            wallpaper.pixels().copy_from_slice(&wp_shadow);
            wallpaper.commit(&mut conn)?;

            state.render_overlay(&mut ov_shadow, sw * 4);
            overlay.pixels().copy_from_slice(&ov_shadow);
            overlay.commit(&mut conn)?;
            dirty = false;
        }

        // 4. Poll LGP socket for input & WM events
        // 33ms timeout targets ~30fps frame rate
        let ret = unsafe { poll(&mut pfd, 1, 33) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }

        if (pfd.revents & POLLIN) != 0 {
            dirty = true;
            loop {
                // Check if data is available before calling recv to avoid blocking
                let mut pfd2 = pollfd {
                    fd: conn.as_raw_fd(),
                    events: POLLIN,
                    revents: 0,
                };
                let ret2 = unsafe { poll(&mut pfd2, 1, 0) };
                if ret2 <= 0 || (pfd2.revents & POLLIN) == 0 {
                    break;
                }

                if let Ok(msg) = conn.recv() {
                    match msg.msg_type {
                        t if t == LgpMessageType::KeyboardKey as u16 => {
                            if let Some(ev) = parse_keyboard_key(&msg.payload) {
                                state.handle_keyboard(ev.key, ev.modifiers, ev.pressed, &mut conn);
                            }
                        }
                        t if t == LgpMessageType::PointerMotion as u16 => {
                            if let Some(ev) = parse_pointer_motion(&msg.payload) {
                                state.handle_pointer_motion(ev.x, ev.y, &mut conn);
                            }
                        }
                        t if t == LgpMessageType::PointerButton as u16 => {
                            if let Some(ev) = parse_pointer_button(&msg.payload) {
                                state.handle_pointer_button(state.cursor_x, state.cursor_y, ev.button, ev.pressed, &mut conn);
                            }
                        }
                        t if t == LgpMessageType::WmSurfaceCreated as u16 => {
                            if let Some(info) = lgp::wm::WmState::parse_surface_created(&msg.payload) {
                                state.wm.on_surface_created(info.surface_id, info.surface_type, info.width, info.height, &mut conn);
                            }
                        }
                        t if t == LgpMessageType::WmSurfaceDestroyed as u16 => {
                            if let Some(sid) = lgp::wm::WmState::parse_surface_destroyed(&msg.payload) {
                                state.wm.on_surface_destroyed(sid, &mut conn);
                            }
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
        }
    }
}
