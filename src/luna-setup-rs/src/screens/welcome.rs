use super::{Screen, WizardState};
use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};
use lunagui::button::Button;
use lunagui::widget::Widget;

pub struct WelcomeScreen {
    next_btn: Button,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        let mut next_btn = Button::new(0, 0, 150, 40, "Next", 0xFF3EE09A);
        next_btn.set_pos(400 - 75, 400); // Centered, arbitrary for now
        Self { next_btn }
    }
}

impl Screen for WelcomeScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        fill_rect_on_slice(pixels, stride, 0, 0, w, h, 0xFF101423); // Background
        draw_text_on_slice(pixels, stride, (w as i32 / 2) - 150, 150, "Welcome to LunaOS Setup", 0xFFFFFFFF);
        
        self.next_btn.set_pos((w as i32 / 2) - 75, (h as i32) - 100);
        self.next_btn.render(pixels, stride, w, h);
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        if self.next_btn.hit_test(x, y) {
            return Some(WizardState::DiskSelect);
        }
        None
    }

    fn on_key(&mut self, _key: u32, _pressed: bool) {}
}
