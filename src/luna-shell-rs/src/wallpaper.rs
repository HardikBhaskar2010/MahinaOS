/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * wallpaper.rs — Wallpaper rendering for luna-shell.
 * Uses luna-wallpaper-rs engine inline (same crate workspace).
 * Handles LRAW video loops and procedural animated fallback.
 */

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

const VIDEO_FILE_PATH: &str = "/usr/share/wallpaper/live.lraw";
const LRAW_HEADER_SIZE: usize = 16;
const MAX_STARS: usize = 180;

#[derive(Clone, Copy)]
struct Star {
    x: u16,
    y: u16,
    brightness: u8,
    twinkle: u8,
}

struct VideoLoop {
    data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    modified_secs: u64,
    byte_len: u64,
}

pub struct WallpaperEngine {
    tick: u32,
    width: u32,
    height: u32,
    stars: Vec<Star>,
    video: Option<VideoLoop>,
    reload_check_counter: u32,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn read_u32_le(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 4 { return None; }
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn modified_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_video_loop(path: &Path) -> Option<VideoLoop> {
    let meta = fs::metadata(path).ok()?;
    let byte_len = meta.len();
    if byte_len < LRAW_HEADER_SIZE as u64 { return None; }

    let data = fs::read(path).ok()?;
    if data.len() < LRAW_HEADER_SIZE || &data[0..4] != b"LRAW" { return None; }

    let width  = read_u32_le(&data[4..8])?;
    let height = read_u32_le(&data[8..12])?;
    let frames = read_u32_le(&data[12..16])?;
    if width == 0 || height == 0 || frames == 0 { return None; }

    let frame_bytes = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    let expected = LRAW_HEADER_SIZE.checked_add(frame_bytes.checked_mul(frames as usize)?)?;
    if data.len() < expected { return None; }

    Some(VideoLoop { data, width, height, frames, modified_secs: modified_secs(&meta), byte_len })
}

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn generate_stars(width: u32, height: u32) -> Vec<Star> {
    let mut seed = width ^ (height << 16) ^ 0x4d41_4849;
    let mut stars = Vec::with_capacity(MAX_STARS);
    for _ in 0..MAX_STARS {
        let x = (lcg_next(&mut seed) % width.max(1)) as u16;
        let y = (lcg_next(&mut seed) % height.max(1)) as u16;
        let brightness = (80 + (lcg_next(&mut seed) % 175)) as u8;
        let twinkle = (lcg_next(&mut seed) & 0x3f) as u8;
        stars.push(Star { x, y, brightness, twinkle });
    }
    stars
}

fn blend_colour(a: u32, b: u32, t: u32) -> u32 {
    let ar = (a >> 16) & 0xff; let ag = (a >> 8) & 0xff; let ab = a & 0xff;
    let br = (b >> 16) & 0xff; let bg = (b >> 8) & 0xff; let bb = b & 0xff;
    let r = (ar * (255 - t) + br * t) / 255;
    let g = (ag * (255 - t) + bg * t) / 255;
    let bl = (ab * (255 - t) + bb * t) / 255;
    (r << 16) | (g << 8) | bl
}

// ── Implementation ───────────────────────────────────────────────────────────

impl WallpaperEngine {
    pub fn new(width: u32, height: u32) -> Self {
        let video = load_video_loop(Path::new(VIDEO_FILE_PATH));
        Self {
            tick: 0,
            width,
            height,
            stars: generate_stars(width, height),
            video,
            reload_check_counter: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);

        // Check for live wallpaper hot-reload every ~2 seconds (60 ticks)
        self.reload_check_counter += 1;
        if self.reload_check_counter >= 60 {
            self.reload_check_counter = 0;
            self.check_reload();
        }
    }

    fn check_reload(&mut self) {
        let path = Path::new(VIDEO_FILE_PATH);
        match fs::metadata(path) {
            Ok(meta) => {
                let modified = modified_secs(&meta);
                let byte_len = meta.len();
                let changed = self.video
                    .as_ref()
                    .map(|v| v.modified_secs != modified || v.byte_len != byte_len)
                    .unwrap_or(true);
                if changed {
                    self.video = load_video_loop(path);
                }
            }
            Err(_) => {
                self.video = None;
            }
        }
    }

    pub fn render(&self, pixels: &mut [u8], stride: u32) {
        if let Some(ref video) = self.video {
            if self.render_video(video, pixels, stride) {
                return;
            }
        }
        self.render_fallback(pixels, stride);
    }

    fn render_video(&self, video: &VideoLoop, pixels: &mut [u8], stride: u32) -> bool {
        let frame_index = (self.tick % video.frames) as usize;
        let frame_bytes = (video.width as usize) * (video.height as usize) * 4;
        let start = LRAW_HEADER_SIZE + frame_index * frame_bytes;
        let end = start + frame_bytes;
        let src = match video.data.get(start..end) {
            Some(s) => s,
            None => return false,
        };

        let dst_w = self.width as usize;
        let dst_h = self.height as usize;
        let src_w = video.width as usize;
        let src_h = video.height as usize;
        let stride = stride as usize;

        if dst_w == 0 || dst_h == 0 || src_w == 0 || src_h == 0 || stride < dst_w * 4 {
            return false;
        }

        // Cover-fit: scale to fill, crop center
        let use_width_scale = dst_w.saturating_mul(src_h) >= dst_h.saturating_mul(src_w);
        let (scaled_w, scaled_h) = if use_width_scale {
            (dst_w, dst_w.saturating_mul(src_h).max(src_h) / src_w.max(1))
        } else {
            (dst_h.saturating_mul(src_w).max(src_w) / src_h.max(1), dst_h)
        };
        let crop_x = scaled_w.saturating_sub(dst_w) / 2;
        let crop_y = scaled_h.saturating_sub(dst_h) / 2;

        for dy in 0..dst_h {
            let sy = if use_width_scale {
                ((dy + crop_y).saturating_mul(src_h) / scaled_h.max(1)).min(src_h - 1)
            } else {
                (dy.saturating_mul(src_h) / dst_h.max(1)).min(src_h - 1)
            };

            let dst_row_start = dy.saturating_mul(stride);
            let dst_row_end = dst_row_start.saturating_add(dst_w * 4);
            let dst_row = match pixels.get_mut(dst_row_start..dst_row_end) {
                Some(r) => r, None => return false,
            };

            for dx in 0..dst_w {
                let sx = if use_width_scale {
                    (dx.saturating_mul(src_w) / dst_w.max(1)).min(src_w - 1)
                } else {
                    ((dx + crop_x).saturating_mul(src_w) / scaled_w.max(1)).min(src_w - 1)
                };
                let src_idx = sy.saturating_mul(src_w).saturating_add(sx).saturating_mul(4);
                let dst_idx = dx.saturating_mul(4);
                if let (Some(s), Some(d)) =
                    (src.get(src_idx..src_idx + 4), dst_row.get_mut(dst_idx..dst_idx + 4))
                {
                    d.copy_from_slice(s);
                }
            }
        }
        true
    }

    fn render_fallback(&self, pixels: &mut [u8], stride: u32) {
        let width  = self.width as usize;
        let height = self.height as usize;
        let stride = stride as usize;
        if width == 0 || height == 0 || stride < width * 4 { return; }

        // Deep navy-to-midnight gradient with aurora banding
        let col_top = 0x050510u32;
        let col_mid = 0x0a0820u32;
        let col_bot = 0x0d0f18u32;

        for y in 0..height {
            let t = (y as u32 * 255) / self.height.max(1);
            let mut color = if t < 128 {
                blend_colour(col_top, col_mid, t * 2)
            } else {
                blend_colour(col_mid, col_bot, (t - 128) * 2)
            };

            // Subtle aurora shimmer band
            let wave = ((y as u32).wrapping_add(self.tick / 6)) % self.height.max(1);
            let band_y = self.height / 4;
            if wave >= band_y && wave < band_y + 40 {
                let pos = wave - band_y;
                let intensity = if pos < 20 { pos * 4 } else { (40 - pos) * 4 };
                color = blend_colour(color, 0x0033aa88, intensity.min(90));
            }
            // Second aurora band
            let wave2 = ((y as u32).wrapping_add(self.tick / 9).wrapping_add(37)) % self.height.max(1);
            let band2_y = self.height * 2 / 5;
            if wave2 >= band2_y && wave2 < band2_y + 30 {
                let pos = wave2 - band2_y;
                let intensity = if pos < 15 { pos * 4 } else { (30 - pos) * 4 };
                color = blend_colour(color, 0x00aa4488, intensity.min(60));
            }

            let row_start = y.saturating_mul(stride);
            let row_end = row_start.saturating_add(width * 4);
            if let Some(row) = pixels.get_mut(row_start..row_end) {
                let bytes = (0xFF000000 | color).to_le_bytes();
                for px in 0..width {
                    row[px * 4..px * 4 + 4].copy_from_slice(&bytes);
                }
            }
        }

        // Render star field
        for star in &self.stars {
            let x = star.x as usize % width.max(1);
            let y = star.y as usize % height.max(1);
            let phase = ((self.tick / 2) + star.twinkle as u32) & 0x3f;
            let twinkle = if phase < 32 { phase * 4 } else { (64 - phase) * 4 };
            let mut brightness = star.brightness as u32;
            brightness = (brightness * (128 + twinkle / 2)) / 255;
            brightness = brightness.min(255);
            let offset = y.saturating_mul(stride).saturating_add(x.saturating_mul(4));
            if let Some(dst) = pixels.get_mut(offset..offset + 4) {
                let c = (0xFF000000u32) | (brightness << 16) | (brightness << 8) | brightness;
                dst.copy_from_slice(&c.to_le_bytes());
            }
        }
    }
}
