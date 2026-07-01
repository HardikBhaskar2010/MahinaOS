use crate::widget::*;
use crate::canvas::Canvas;
use crate::color::COLOR_TEXT;

pub struct Label {
    pub base: WidgetBase,
    pub text: String,
    pub color: u32,
    pub align: LabelAlign,
    pub font_size: u32,
}

pub enum LabelAlign {
    Left,
    Center,
    Right,
}

impl Label {
    pub fn new(x: i32, y: i32, width: u32, height: u32, text: &str) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            text: text.to_string(),
            color: COLOR_TEXT,
            align: LabelAlign::Left,
            font_size: 16,
        }
    }

    pub fn colored(x: i32, y: i32, width: u32, height: u32, text: &str, color: u32) -> Self {
        let mut l = Self::new(x, y, width, height, text);
        l.color = color;
        l
    }

    pub fn centered(x: i32, y: i32, width: u32, height: u32, text: &str) -> Self {
        let mut l = Self::new(x, y, width, height, text);
        l.align = LabelAlign::Center;
        l
    }
}

impl Widget for Label {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; }

    fn render(&self, pixels: &mut [u8], stride: u32, _surface_w: u32, _surface_h: u32) {
        let mut canvas = Canvas::from_slice(pixels, stride / 4, _surface_h, stride);
        canvas.push_clip(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32);
        let text_w = (self.text.len() as u32) * 8;
        let tx = match self.align {
            LabelAlign::Left => self.base.x + 4,
            LabelAlign::Center => self.base.x + (self.base.width as i32 - text_w as i32) / 2,
            LabelAlign::Right => self.base.x + self.base.width as i32 - text_w as i32 - 4,
        };
        let ty = self.base.y + (self.base.height as i32 - 16) / 2;
        canvas.draw_text(tx, ty, &self.text, self.color);
        canvas.pop_clip();
    }

    fn hit_test(&self, x: i32, y: i32) -> bool { self.base.hit_test(x, y) }
}
