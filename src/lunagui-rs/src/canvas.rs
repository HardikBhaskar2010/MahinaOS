use crate::color::alpha_blend;
use crate::font::FONT8X16;

const MAX_CLIP_STACK: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct ClipRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct Canvas {
    pub pixels: *mut u8,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub owned: bool,
    clip_stack: [ClipRect; MAX_CLIP_STACK],
    clip_count: usize,
}

impl Canvas {
    pub fn new(pixels: *mut u8, width: u32, height: u32, stride: u32) -> Self {
        let mut c = Self {
            pixels,
            width,
            height,
            stride,
            owned: false,
            clip_stack: [ClipRect { x: 0, y: 0, w: 0, h: 0 }; MAX_CLIP_STACK],
            clip_count: 0,
        };
        c.push_clip(0, 0, width as i32, height as i32);
        c
    }

    pub fn from_slice(slice: &mut [u8], width: u32, height: u32, stride: u32) -> Self {
        Self::new(slice.as_mut_ptr(), width, height, stride)
    }

    pub fn get_clip(&self) -> Option<ClipRect> {
        if self.clip_count == 0 { return None; }
        Some(self.clip_stack[self.clip_count - 1])
    }

    pub fn push_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if self.clip_count >= MAX_CLIP_STACK { return; }
        let (nx, ny, nw, nh) = if self.clip_count > 0 {
            let top = self.clip_stack[self.clip_count - 1];
            let nx = if x > top.x { x } else { top.x };
            let ny = if y > top.y { y } else { top.y };
            let nw = if x + w < top.x + top.w { x + w } else { top.x + top.w } - nx;
            let nh = if y + h < top.y + top.h { y + h } else { top.y + top.h } - ny;
            (nx, ny, if nw < 0 { 0 } else { nw }, if nh < 0 { 0 } else { nh })
        } else {
            (x, y, w, h)
        };
        self.clip_stack[self.clip_count] = ClipRect { x: nx, y: ny, w: nw, h: nh };
        self.clip_count += 1;
    }

    pub fn pop_clip(&mut self) {
        if self.clip_count <= 1 { return; }
        self.clip_count -= 1;
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        if self.pixels.is_null() { return; }
        let Some(clip) = self.get_clip() else { return };
        let nx = if x > clip.x { x } else { clip.x };
        let ny = if y > clip.y { y } else { clip.y };
        let nw = if x + w < clip.x + clip.w { x + w } else { clip.x + clip.w } - nx;
        let nh = if y + h < clip.y + clip.h { y + h } else { clip.y + clip.h } - ny;
        if nw <= 0 || nh <= 0 { return; }

        let a = (color >> 24) & 0xFF;
        let is_opaque = a == 0xFF;
        let is_transparent = a == 0x00;

        unsafe {
            for py in ny..ny + nh {
                let row = self.pixels.add((py as u32 * self.stride) as usize);
                for px in nx..nx + nw {
                    let p = row.add((px as u32 * 4) as usize);
                    if is_opaque {
                        std::ptr::write(p as *mut u32, color);
                    } else if !is_transparent {
                        let dst = std::ptr::read(p as *const u32);
                        std::ptr::write(p as *mut u32, alpha_blend(color, dst));
                    }
                }
            }
        }
    }

    pub fn draw_rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        if w <= 0 || h <= 0 { return; }
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h - 1, w, 1, color);
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w - 1, y, 1, h, color);
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: u32) {
        if self.pixels.is_null() { return; }
        let a = (color >> 24) & 0xFF;
        let is_opaque = a == 0xFF;
        let fg = if is_opaque { color } else { color | 0xFF000000 };
        let glyphs = FONT8X16;
        for (col, ch) in text.bytes().enumerate() {
            let gidx = ch as usize;
            if gidx >= glyphs.len() / 16 { continue; }
            let gdata = &glyphs[gidx * 16..gidx * 16 + 16];
            for row in 0..16 {
                let line = gdata[row];
                for col_bit in 0..8 {
                    if line & (0x80 >> col_bit) != 0 {
                        let px = x + col as i32 * 8 + col_bit;
                        let py = y + row as i32;
                        if px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32 {
                            let offset = py as u32 * self.stride + px as u32 * 4;
                            unsafe {
                                let p = self.pixels.add(offset as usize);
                                if is_opaque {
                                    std::ptr::write(p as *mut u32, fg);
                                } else {
                                    let dst = std::ptr::read(p as *const u32);
                                    std::ptr::write(p as *mut u32, alpha_blend(color, dst));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.fill_rect(0, 0, self.width as i32, self.height as i32, color);
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        if self.owned && !self.pixels.is_null() {
            unsafe {
                libc::free(self.pixels as *mut libc::c_void);
            }
        }
    }
}

pub fn draw_text_on_slice(pixels: &mut [u8], stride: u32, surface_w: u32, x: i32, y: i32, text: &str, color: u32) {
    let a = (color >> 24) & 0xFF;
    let is_opaque = a == 0xFF;
    for (col, ch) in text.bytes().enumerate() {
        let gidx = ch as usize;
        if gidx >= 4096 / 16 { continue; }
        let gdata = &FONT8X16[gidx * 16..gidx * 16 + 16];
        for row in 0..16i32 {
            let line = gdata[row as usize];
            for col_bit in 0..8i32 {
                if line & (0x80 >> col_bit) != 0 {
                    let px = x + col as i32 * 8 + col_bit;
                    let py = y + row;
                    if px >= 0 && px < surface_w as i32 && py >= 0 {
                        let offset = (py as u32 * stride + px as u32 * 4) as usize;
                        if offset + 4 <= pixels.len() {
                            if is_opaque {
                                pixels[offset..offset + 4].copy_from_slice(&color.to_le_bytes());
                            } else {
                                let dst = u32::from_le_bytes([
                                    pixels[offset], pixels[offset + 1],
                                    pixels[offset + 2], pixels[offset + 3],
                                ]);
                                let blended = alpha_blend(color, dst);
                                pixels[offset..offset + 4].copy_from_slice(&blended.to_le_bytes());
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn fill_rect_on_slice(pixels: &mut [u8], stride: u32, surface_w: u32, _surface_h: u32,
    x: i32, y: i32, w: i32, h: i32, color: u32)
{
    let a = (color >> 24) & 0xFF;
    let is_opaque = a == 0xFF;
    for py in y..y + h {
        if py < 0 { continue; }
        for px in x..x + w {
            if px < 0 || px >= surface_w as i32 { continue; }
            let offset = (py as u32 * stride + px as u32 * 4) as usize;
            if offset + 4 <= pixels.len() {
                if is_opaque {
                    pixels[offset..offset + 4].copy_from_slice(&color.to_le_bytes());
                } else {
                    let dst = u32::from_le_bytes([
                        pixels[offset], pixels[offset + 1],
                        pixels[offset + 2], pixels[offset + 3],
                    ]);
                    let blended = alpha_blend(color, dst);
                    pixels[offset..offset + 4].copy_from_slice(&blended.to_le_bytes());
                }
            }
        }
    }
}
