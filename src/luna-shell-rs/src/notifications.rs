/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * notifications.rs — Toast notification system for luna-shell.
 * Notifications appear in the top-right corner and auto-dismiss.
 */

const MAX_NOTIFICATIONS: usize = 5;
const NOTIFICATION_LIFETIME_TICKS: u32 = 150; // ~5 seconds at ~30fps
const NOTIFICATION_W: i32 = 240;
const NOTIFICATION_H: i32 = 40;
const NOTIFICATION_MARGIN: i32 = 8;
const NOTIFICATION_TOP: i32 = 36; // Below taskbar

use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};

#[derive(Clone)]
pub struct Notification {
    pub message: String,
    pub ticks_left: u32,
    pub color: u32, // Accent color for the left border
}

pub struct NotificationManager {
    queue: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn push(&mut self, message: impl Into<String>, color: u32) {
        if self.queue.len() >= MAX_NOTIFICATIONS {
            self.queue.remove(0);
        }
        self.queue.push(Notification {
            message: message.into(),
            ticks_left: NOTIFICATION_LIFETIME_TICKS,
            color,
        });
    }

    pub fn push_info(&mut self, message: impl Into<String>) {
        self.push(message, 0xFF6B7FD4); // Accent blue
    }

    pub fn push_warning(&mut self, message: impl Into<String>) {
        self.push(message, 0xFFE0973E); // Orange
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.push(message, 0xFFE03E3E); // Red
    }

    pub fn push_success(&mut self, message: impl Into<String>) {
        self.push(message, 0xFF3EE09A); // Green
    }

    /// Call once per frame to advance animation timers.
    pub fn tick(&mut self) {
        self.queue.retain_mut(|n| {
            if n.ticks_left > 0 {
                n.ticks_left -= 1;
                true
            } else {
                false
            }
        });
    }

    /// Render all active notifications onto the overlay surface.
    pub fn render(&self, pixels: &mut [u8], stride: u32, sw: u32, sh: u32) {
        let screen_w = sw as i32;

        for (i, notif) in self.queue.iter().enumerate() {
            let x = screen_w - NOTIFICATION_W - NOTIFICATION_MARGIN;
            let y = NOTIFICATION_TOP + (NOTIFICATION_H + NOTIFICATION_MARGIN) * i as i32;

            // Fade alpha based on remaining ticks
            let alpha = if notif.ticks_left < 30 {
                ((notif.ticks_left as u32 * 200) / 30) as u32
            } else {
                200u32
            };

            let bg = (alpha << 24) | 0x001218_20;
            fill_rect_on_slice(pixels, stride, sw, sh, x, y, NOTIFICATION_W, NOTIFICATION_H, bg);

            // Left accent border
            fill_rect_on_slice(pixels, stride, sw, sh, x, y, 3, NOTIFICATION_H, notif.color);

            // Message text (truncated to fit)
            let max_chars = ((NOTIFICATION_W - 16) / 8) as usize;
            let text = if notif.message.len() > max_chars {
                &notif.message[..max_chars]
            } else {
                &notif.message
            };
            let text_color = (alpha << 24) | 0x00EEEEf4;
            draw_text_on_slice(pixels, stride, sw, x + 10, y + 12, text, text_color);
        }
    }
}
