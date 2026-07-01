pub trait Widget {
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn set_pos(&mut self, x: i32, y: i32);
    fn set_size(&mut self, width: u32, height: u32);
    fn render(&self, pixels: &mut [u8], stride: u32, surface_w: u32, surface_h: u32);
    fn hit_test(&self, x: i32, y: i32) -> bool;
    fn on_click(&mut self, _x: i32, _y: i32) {}
    fn on_mouse_down(&mut self, _x: i32, _y: i32) {}
    fn on_mouse_up(&mut self, _x: i32, _y: i32) {}
    fn on_key(&mut self, _key: u32, _pressed: bool) {}
    fn padding(&self) -> (u32, u32, u32, u32) { (0, 0, 0, 0) }
    fn margin(&self) -> (u32, u32, u32, u32) { (0, 0, 0, 0) }
}

pub struct WidgetBase {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl WidgetBase {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
    pub fn hit_test(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width as i32
            && py >= self.y && py < self.y + self.height as i32
    }
}

pub enum Cursor {
    Default,
    Pointer,
    Text,
    Move,
    Hand,
}
