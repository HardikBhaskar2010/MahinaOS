/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * keybinds.rs — Centralized keyboard shortcut definitions for luna-shell.
 */

use lgp::protocol::{LGP_MOD_SUPER, LGP_MOD_SHIFT, LGP_MOD_CTRL, LGP_MOD_ALT};

/// Linux evdev key codes (matches kernel's linux/input-event-codes.h)
pub mod keys {
    pub const KEY_1: u32 = 0x02;
    pub const KEY_2: u32 = 0x03;
    pub const KEY_3: u32 = 0x04;
    pub const KEY_4: u32 = 0x05;
    pub const KEY_5: u32 = 0x06;
    pub const KEY_6: u32 = 0x07;
    pub const KEY_Q: u32 = 0x10;
    pub const KEY_W: u32 = 0x11;
    pub const KEY_E: u32 = 0x12;
    pub const KEY_T: u32 = 0x14;
    pub const KEY_F: u32 = 0x21;
    pub const KEY_J: u32 = 0x24;
    pub const KEY_K: u32 = 0x25;
    pub const KEY_L: u32 = 0x26;
    pub const KEY_V: u32 = 0x2F;
    pub const KEY_UP: u32 = 0x67;
    pub const KEY_DOWN: u32 = 0x6C;
    pub const KEY_LEFT: u32 = 0x69;
    pub const KEY_RIGHT: u32 = 0x6A;
    pub const KEY_TAB: u32 = 0x0F;
    pub const KEY_ENTER: u32 = 0x1C;
    pub const KEY_SPACE: u32 = 0x39;
    pub const KEY_ESC: u32 = 0x01;
    pub const KEY_BACKSPACE: u32 = 0x0E;
    pub const KEY_0: u32 = 0x0B;
    pub const KEY_9: u32 = 0x0A;
    pub const KEY_8: u32 = 0x09;
    pub const KEY_7: u32 = 0x08;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    SwitchWorkspace(usize),         // Super+1..6
    MoveWindowToWorkspace(usize),   // Super+Shift+1..6
    OpenTerminal,                   // Super+Enter
    OpenLauncher,                   // Super+Space
    CloseWindow,                    // Super+Q
    ToggleFloating,                 // Super+V
    ToggleFullscreen,               // Super+F
    FocusNext,                      // Super+K or Alt+Tab
    FocusPrev,                      // Super+J
    SnapLeft,                       // Super+Left
    SnapRight,                      // Super+Right
    SnapUp,                         // Super+Up (maximize)
    SnapDown,                       // Super+Down (restore)
    ResizeShrink,                   // Super+Ctrl+Left/Down
    ResizeGrow,                     // Super+Ctrl+Right/Up
    LockScreen,                     // Super+L
    Quit,                           // Super+Shift+Q
    None,
}

pub fn dispatch(key: u32, modifiers: u32) -> ShellAction {
    let super_ = (modifiers & LGP_MOD_SUPER) != 0;
    let shift  = (modifiers & LGP_MOD_SHIFT) != 0;
    let ctrl   = (modifiers & LGP_MOD_CTRL)  != 0;
    let alt    = (modifiers & LGP_MOD_ALT)   != 0;

    // Workspace switching: Super+1..6
    if super_ && !shift && !ctrl {
        match key {
            keys::KEY_1 => return ShellAction::SwitchWorkspace(0),
            keys::KEY_2 => return ShellAction::SwitchWorkspace(1),
            keys::KEY_3 => return ShellAction::SwitchWorkspace(2),
            keys::KEY_4 => return ShellAction::SwitchWorkspace(3),
            keys::KEY_5 => return ShellAction::SwitchWorkspace(4),
            keys::KEY_6 => return ShellAction::SwitchWorkspace(5),
            _ => {}
        }
    }

    // Move window to workspace: Super+Shift+1..6
    if super_ && shift && !ctrl {
        match key {
            keys::KEY_1 => return ShellAction::MoveWindowToWorkspace(0),
            keys::KEY_2 => return ShellAction::MoveWindowToWorkspace(1),
            keys::KEY_3 => return ShellAction::MoveWindowToWorkspace(2),
            keys::KEY_4 => return ShellAction::MoveWindowToWorkspace(3),
            keys::KEY_5 => return ShellAction::MoveWindowToWorkspace(4),
            keys::KEY_6 => return ShellAction::MoveWindowToWorkspace(5),
            keys::KEY_Q => return ShellAction::Quit,
            _ => {}
        }
    }

    // Super combos
    if super_ && !shift && !ctrl && !alt {
        match key {
            keys::KEY_ENTER => return ShellAction::OpenTerminal,
            keys::KEY_SPACE => return ShellAction::OpenLauncher,
            keys::KEY_Q     => return ShellAction::CloseWindow,
            keys::KEY_V     => return ShellAction::ToggleFloating,
            keys::KEY_F     => return ShellAction::ToggleFullscreen,
            keys::KEY_J     => return ShellAction::FocusPrev,
            keys::KEY_K     => return ShellAction::FocusNext,
            keys::KEY_L     => return ShellAction::LockScreen,
            keys::KEY_LEFT  => return ShellAction::SnapLeft,
            keys::KEY_RIGHT => return ShellAction::SnapRight,
            keys::KEY_UP    => return ShellAction::SnapUp,
            keys::KEY_DOWN  => return ShellAction::SnapDown,
            _ => {}
        }
    }

    // Super+Ctrl resize
    if super_ && ctrl && !shift {
        match key {
            keys::KEY_LEFT | keys::KEY_DOWN => return ShellAction::ResizeShrink,
            keys::KEY_RIGHT | keys::KEY_UP  => return ShellAction::ResizeGrow,
            _ => {}
        }
    }

    // Alt+Tab
    if alt && key == keys::KEY_TAB {
        return ShellAction::FocusNext;
    }

    ShellAction::None
}
