use crate::widget::*;

pub struct HBox {
    pub base: WidgetBase,
    pub children: Vec<Box<dyn Widget>>,
    pub spacing: u32,
}

impl HBox {
    pub fn new(x: i32, y: i32, width: u32, height: u32, spacing: u32) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            children: Vec::new(),
            spacing,
        }
    }

    pub fn add(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
        self.layout();
    }

    fn layout(&mut self) {
        let total_spacing = if self.children.len() > 1 {
            self.spacing * (self.children.len() as u32 - 1)
        } else {
            0
        };
        let child_w = if self.children.len() > 0 {
            (self.base.width - total_spacing) / self.children.len() as u32
        } else {
            0
        };
        let mut cx = self.base.x;
        for child in &mut self.children {
            child.set_pos(cx, self.base.y);
            child.set_size(child_w, self.base.height);
            cx += child_w as i32 + self.spacing as i32;
        }
    }
}

impl Widget for HBox {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; self.layout(); }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; self.layout(); }

    fn render(&self, pixels: &mut [u8], stride: u32, surface_w: u32, surface_h: u32) {
        for child in &self.children {
            child.render(pixels, stride, surface_w, surface_h);
        }
    }

    fn hit_test(&self, x: i32, y: i32) -> bool {
        self.children.iter().any(|c| c.hit_test(x, y))
    }

    fn on_click(&mut self, x: i32, y: i32) {
        for child in &mut self.children {
            if child.hit_test(x, y) {
                child.on_click(x, y);
            }
        }
    }

    fn on_mouse_down(&mut self, x: i32, y: i32) {
        for child in &mut self.children {
            if child.hit_test(x, y) {
                child.on_mouse_down(x, y);
            }
        }
    }

    fn on_mouse_up(&mut self, x: i32, y: i32) {
        for child in &mut self.children {
            child.on_mouse_up(x, y);
        }
    }
}

pub struct VBox {
    pub base: WidgetBase,
    pub children: Vec<Box<dyn Widget>>,
    pub spacing: u32,
}

impl VBox {
    pub fn new(x: i32, y: i32, width: u32, height: u32, spacing: u32) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            children: Vec::new(),
            spacing,
        }
    }

    pub fn add(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
        self.layout();
    }

    fn layout(&mut self) {
        let total_spacing = if self.children.len() > 1 {
            self.spacing * (self.children.len() as u32 - 1)
        } else {
            0
        };
        let child_h = if self.children.len() > 0 {
            (self.base.height - total_spacing) / self.children.len() as u32
        } else {
            0
        };
        let mut cy = self.base.y;
        for child in &mut self.children {
            child.set_pos(self.base.x, cy);
            child.set_size(self.base.width, child_h);
            cy += child_h as i32 + self.spacing as i32;
        }
    }
}

impl Widget for VBox {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; self.layout(); }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; self.layout(); }

    fn render(&self, pixels: &mut [u8], stride: u32, surface_w: u32, surface_h: u32) {
        for child in &self.children {
            child.render(pixels, stride, surface_w, surface_h);
        }
    }

    fn hit_test(&self, x: i32, y: i32) -> bool {
        self.children.iter().any(|c| c.hit_test(x, y))
    }

    fn on_click(&mut self, x: i32, y: i32) {
        for child in &mut self.children {
            child.on_click(x, y);
        }
    }

    fn on_mouse_down(&mut self, x: i32, y: i32) {
        for child in &mut self.children {
            if child.hit_test(x, y) {
                child.on_mouse_down(x, y);
            }
        }
    }

    fn on_mouse_up(&mut self, x: i32, y: i32) {
        for child in &mut self.children {
            child.on_mouse_up(x, y);
        }
    }
}
