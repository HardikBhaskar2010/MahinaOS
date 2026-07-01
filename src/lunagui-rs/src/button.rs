use crate::widget::*;
use crate::canvas::Canvas;
use crate::color::{COLOR_RAISED, COLOR_HOVER, COLOR_PRESSED, COLOR_ACCENT, COLOR_TEXT};

pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
}

pub struct Button {
    pub base: WidgetBase,
    pub text: String,
    pub state: ButtonState,
    pub on_activate: Option<Box<dyn FnMut()>>,
    pub accent: bool,
}

impl Button {
    pub fn new(x: i32, y: i32, width: u32, height: u32, text: &str) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            text: text.to_string(),
            state: ButtonState::Normal,
            on_activate: None,
            accent: false,
        }
    }

    pub fn accent(x: i32, y: i32, width: u32, height: u32, text: &str) -> Self {
        let mut b = Self::new(x, y, width, height, text);
        b.accent = true;
        b
    }
}

impl Widget for Button {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; }

    fn render(&self, pixels: &mut [u8], stride: u32, _surface_w: u32, surface_h: u32) {
        let mut canvas = Canvas::from_slice(pixels, stride / 4, surface_h, stride);
        canvas.push_clip(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32);

        let bg = match self.state {
            ButtonState::Normal => if self.accent { COLOR_ACCENT } else { COLOR_RAISED },
            ButtonState::Hover => if self.accent { COLOR_ACCENT } else { COLOR_HOVER },
            ButtonState::Pressed => COLOR_PRESSED,
        };
        canvas.fill_rect(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32, bg);

        if self.accent {
            canvas.draw_rect_outline(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32, 0xFF8A9FFF);
        }

        let text_w = (self.text.len() as u32) * 8;
        let tx = self.base.x + (self.base.width as i32 - text_w as i32) / 2;
        let ty = self.base.y + (self.base.height as i32 - 16) / 2;
        let text_color = if self.accent { 0xFFFFFFFF } else { COLOR_TEXT };
        canvas.draw_text(tx, ty, &self.text, text_color);

        canvas.pop_clip();
    }

    fn hit_test(&self, x: i32, y: i32) -> bool { self.base.hit_test(x, y) }

    fn on_mouse_down(&mut self, _x: i32, _y: i32) {
        self.state = ButtonState::Pressed;
    }

    fn on_mouse_up(&mut self, _x: i32, _y: i32) {
        self.state = if self.base.hit_test(_x, _y) { ButtonState::Hover } else { ButtonState::Normal };
        if let Some(ref mut cb) = self.on_activate {
            cb();
        }
    }
}
