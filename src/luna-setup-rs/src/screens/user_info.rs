use super::{render_wizard_chrome, Screen, WizardState, COLOR_CARD_INNER, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SEC};
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};
use lunagui::Button;
use lunagui::TextField;
use lunagui::Widget;

pub struct UserInfoScreen {
    next_btn: Button,
    back_btn: Button,
    pub username_field: TextField,
    pub password_field: TextField,
    pub hostname_field: TextField,
}

impl UserInfoScreen {
    pub fn new() -> Self {
        let mut user_f = TextField::new(0, 0, 360, 32);
        user_f.text = "user".to_string();
        user_f.cursor_pos = 4;

        let mut pass_f = TextField::new(0, 0, 360, 32);
        pass_f.password_mode = true;
        pass_f.placeholder = "Enter secure password".to_string();

        let mut host_f = TextField::new(0, 0, 360, 32);
        host_f.text = "mahina-box".to_string();
        host_f.cursor_pos = 10;

        Self {
            next_btn: Button::accent(0, 0, 180, 36, "Install MahinaOS ->"),
            back_btn: Button::new(0, 0, 120, 36, "<- Back"),
            username_field: user_f,
            password_field: pass_f,
            hostname_field: host_f,
        }
    }
}

impl Screen for UserInfoScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        render_wizard_chrome(pixels, stride, w, h, 3, "Identity & System Identity");

        let card_w = 680i32;
        let card_h = 500i32;
        let card_x = (w as i32 - card_w) / 2;
        let card_y = (h as i32 - card_h) / 2;

        let content_x = card_x + 36;
        let mut y = card_y + 86;

        draw_text_on_slice(pixels, stride, w, content_x, y, "Configure your primary user identity and machine hostname:", COLOR_TEXT_SEC);
        y += 26;

        // Username
        draw_text_on_slice(pixels, stride, w, content_x, y, "Primary Username (POSIX Owner)", COLOR_TEXT_PRIMARY);
        y += 18;
        self.username_field.set_pos(content_x, y);
        self.username_field.render(pixels, stride, w, h);
        draw_text_on_slice(pixels, stride, w, content_x + 380, y + 8, "Owner of /home/<user> (0700)", COLOR_TEXT_MUTED);
        y += 44;

        // Password
        draw_text_on_slice(pixels, stride, w, content_x, y, "Account Password", COLOR_TEXT_PRIMARY);
        y += 18;
        self.password_field.set_pos(content_x, y);
        self.password_field.render(pixels, stride, w, h);
        draw_text_on_slice(pixels, stride, w, content_x + 380, y + 8, "SHA-512 crypt in /etc/shadow", COLOR_TEXT_MUTED);
        y += 44;

        // Hostname
        draw_text_on_slice(pixels, stride, w, content_x, y, "System Hostname", COLOR_TEXT_PRIMARY);
        y += 18;
        self.hostname_field.set_pos(content_x, y);
        self.hostname_field.render(pixels, stride, w, h);
        draw_text_on_slice(pixels, stride, w, content_x + 380, y + 8, "Written to /etc/hostname", COLOR_TEXT_MUTED);
        y += 44;

        // Security boundary card
        let notice_w = card_w - 72;
        let notice_h = 54;
        fill_rect_on_slice(pixels, stride, w, h, content_x - 1, y - 1, notice_w + 2, notice_h + 2, COLOR_BORDER);
        fill_rect_on_slice(pixels, stride, w, h, content_x, y, notice_w, notice_h, COLOR_CARD_INNER);

        draw_text_on_slice(pixels, stride, w, content_x + 12, y + 10, "SECURITY: LunaAI runs in Tier 5 as unprivileged user (UID 950).", COLOR_TEXT_SEC);
        draw_text_on_slice(pixels, stride, w, content_x + 12, y + 28, "It cannot modify system files or access your data without Capability Tokens.", COLOR_TEXT_MUTED);

        // Buttons
        self.back_btn.set_pos(content_x, card_y + card_h - 24 - 36);
        self.back_btn.render(pixels, stride, w, h);

        self.next_btn.set_pos(card_x + card_w - 36 - 180, card_y + card_h - 24 - 36);
        self.next_btn.render(pixels, stride, w, h);
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        self.username_field.focused = self.username_field.hit_test(x, y);
        if self.username_field.focused {
            self.username_field.on_click(x, y);
        }

        self.password_field.focused = self.password_field.hit_test(x, y);
        if self.password_field.focused {
            self.password_field.on_click(x, y);
        }

        self.hostname_field.focused = self.hostname_field.hit_test(x, y);
        if self.hostname_field.focused {
            self.hostname_field.on_click(x, y);
        }

        if self.back_btn.hit_test(x, y) {
            return Some(WizardState::DiskSelect);
        }
        if self.next_btn.hit_test(x, y) {
            if !self.username_field.text.trim().is_empty() && !self.password_field.text.is_empty() {
                return Some(WizardState::Progress);
            }
        }
        None
    }

    fn on_key(&mut self, key: u32, pressed: bool) {
        if self.username_field.focused {
            self.username_field.on_key(key, pressed);
        } else if self.password_field.focused {
            self.password_field.on_key(key, pressed);
        } else if self.hostname_field.focused {
            self.hostname_field.on_key(key, pressed);
        }
    }
}
