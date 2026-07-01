use crate::widget::*;
use crate::canvas::Canvas;

pub type CanvasRenderFn = Box<dyn Fn(&Canvas)>;

pub struct CanvasWidget {
    pub base: WidgetBase,
    pub render_fn: Option<CanvasRenderFn>,
    pub click_zones: Vec<ClickZone>,
}

pub struct ClickZone {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub id: u32,
}

impl CanvasWidget {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            base: WidgetBase::new(x, y, width, height),
            render_fn: None,
            click_zones: Vec::new(),
        }
    }

    pub fn set_render(&mut self, f: CanvasRenderFn) {
        self.render_fn = Some(f);
    }

    pub fn hit_zone(&self, px: i32, py: i32) -> Option<u32> {
        for zone in &self.click_zones {
            if px >= zone.x && px < zone.x + zone.w
                && py >= zone.y && py < zone.y + zone.h
            {
                return Some(zone.id);
            }
        }
        None
    }
}

impl Widget for CanvasWidget {
    fn x(&self) -> i32 { self.base.x }
    fn y(&self) -> i32 { self.base.y }
    fn width(&self) -> u32 { self.base.width }
    fn height(&self) -> u32 { self.base.height }
    fn set_pos(&mut self, x: i32, y: i32) { self.base.x = x; self.base.y = y; }
    fn set_size(&mut self, width: u32, height: u32) { self.base.width = width; self.base.height = height; }

    fn render(&self, pixels: &mut [u8], stride: u32, _surface_w: u32, _surface_h: u32) {
        if let Some(ref f) = self.render_fn {
            let canvas = Canvas::from_slice(pixels, stride / 4, _surface_h, stride);
            f(&canvas);
        }
    }

    fn hit_test(&self, x: i32, y: i32) -> bool { self.base.hit_test(x, y) }
}
