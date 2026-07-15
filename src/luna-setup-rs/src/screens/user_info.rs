use super::{Screen, WizardState};
use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use lunagui::button::Button;
use lunagui::textfield::TextField;
use lunagui::widget::Widget;

pub struct UserInfoScreen {
    next_btn: Button,
    back_btn: Button,
    pub username_field: TextField,
    pub password_field: TextField,
    pub hostname_field: TextField,
}

impl UserInfoScreen {
    pub fn new() -> Self {
        Self {
            next_btn: Button::new(0, 0, 150, 40, "Install", 0xFFE03E8A),
            back_btn: Button::new(0, 0, 150, 40, "Back", 0xFF6B7FD4),
            username_field: TextField::new(0, 0, 300, 40),
            password_field: TextField::new(0, 0, 300, 40),
            hostname_field: TextField::new(0, 0, 300, 40),
        }
    }
}

impl Screen for UserInfoScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        fill_rect_on_slice(pixels, stride, 0, 0, w, h, 0xFF101423); // Background
        draw_text_on_slice(pixels, stride, (w as i32 / 2) - 150, 80, "User Configuration", 0xFFFFFFFF);
        
        let cx = (w as i32 / 2) - 150;
        
        draw_text_on_slice(pixels, stride, cx, 130, "Username:", 0xFFFFFFFF);
        self.username_field.set_pos(cx, 160);
        self.username_field.render(pixels, stride, w, h);

        draw_text_on_slice(pixels, stride, cx, 220, "Password:", 0xFFFFFFFF);
        self.password_field.set_pos(cx, 250);
        // Note: TextField doesn't support mask by default in lunagui, we'd add it later.
        self.password_field.render(pixels, stride, w, h);

        draw_text_on_slice(pixels, stride, cx, 310, "Hostname:", 0xFFFFFFFF);
        self.hostname_field.set_pos(cx, 340);
        self.hostname_field.render(pixels, stride, w, h);

        self.back_btn.set_pos((w as i32 / 2) - 160, (h as i32) - 100);
        self.back_btn.render(pixels, stride, w, h);
        
        self.next_btn.set_pos((w as i32 / 2) + 10, (h as i32) - 100);
        self.next_btn.render(pixels, stride, w, h);
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        self.username_field.on_mouse_down(x, y);
        self.password_field.on_mouse_down(x, y);
        self.hostname_field.on_mouse_down(x, y);

        if self.back_btn.hit_test(x, y) {
            return Some(WizardState::DiskSelect);
        }
        if self.next_btn.hit_test(x, y) {
            if !self.username_field.text.is_empty() && !self.password_field.text.is_empty() {
                return Some(WizardState::Progress);
            }
        }
        None
    }

    fn on_key(&mut self, key: u32, pressed: bool) {
        self.username_field.on_key(key, pressed);
        self.password_field.on_key(key, pressed);
        self.hostname_field.on_key(key, pressed);
    }
}
