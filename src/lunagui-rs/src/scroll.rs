use crate::widget::*;
use crate::canvas::Canvas;

pub struct Scroll {
    pub base: WidgetBase,
    pub content: Option<Box<dyn Widget>>,
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub content_w: u32,
    pub content_h: u32,
}

impl Scroll {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            content: None,
            scroll_x: 0,
            scroll_y: 0,
            content_w: width,
            content_h: height,
        }
    }

    pub fn set_content(&mut self, child: Box<dyn Widget>) {
        self.content_w = child.width();
        self.content_h = child.height();
        self.content = Some(child);
    }
}

impl Widget for Scroll {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; }

    fn render(&self, pixels: &mut [u8], stride: u32, _surface_w: u32, surface_h: u32) {
        let mut canvas = Canvas::from_slice(pixels, stride / 4, surface_h, stride);
        canvas.push_clip(self.base.x, self.base.y, self.base.width as i32, self.base.height as i32);

        if let Some(ref child) = self.content {
            child.render(pixels, stride, _surface_w, surface_h);
        }

        canvas.pop_clip();
    }

    fn hit_test(&self, x: i32, y: i32) -> bool { self.base.hit_test(x, y) }
    fn on_click(&mut self, x: i32, y: i32) {
        if let Some(ref mut child) = self.content {
            child.on_click(x, y);
        }
    }
    fn on_mouse_down(&mut self, x: i32, y: i32) {
        if let Some(ref mut child) = self.content {
            child.on_mouse_down(x, y);
        }
    }
    fn on_mouse_up(&mut self, x: i32, y: i32) {
        if let Some(ref mut child) = self.content {
            child.on_mouse_up(x, y);
        }
    }
}
