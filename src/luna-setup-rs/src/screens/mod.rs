pub mod welcome;
pub mod disk_select;
pub mod user_info;
pub mod progress;

use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};

// Visual design tokens — Hardware-machined utilitarian luxury
pub const COLOR_BG: u32 = 0xFF0A0A0F;          // Deepest OLED void black
pub const COLOR_CARD: u32 = 0xFF14141E;        // Card container
pub const COLOR_CARD_INNER: u32 = 0xFF1C1C28;  // Input / tile background
pub const COLOR_BORDER: u32 = 0xFF2A2A3C;      // Subtle 1px structural outline
pub const COLOR_BORDER_ACCENT: u32 = 0xFF7E8CE0; // Muted Periwinkle active focus
pub const COLOR_TEXT_PRIMARY: u32 = 0xFFFFFFFF; // Crisp white
pub const COLOR_TEXT_SEC: u32 = 0xFF9E9EA8;    // Secondary body text
pub const COLOR_TEXT_MUTED: u32 = 0xFF5E5E6C;  // Muted breadcrumbs
pub const COLOR_ACCENT: u32 = 0xFF7E8CE0;      // Active step
pub const COLOR_SUCCESS: u32 = 0xFF5BB98C;     // Completed step / success
pub const COLOR_WARN: u32 = 0xFFE5A440;        // Warning / formatting notice
pub const COLOR_ERROR: u32 = 0xFFE55C5C;       // Error / refusal

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WizardState {
    Welcome,
    DiskSelect,
    UserInfo,
    Progress,
    Done,
}

pub trait Screen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32);
    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState>;
    fn on_key(&mut self, key: u32, pressed: bool);
}

/// Renders the signature MahinaOS double-bezel card frame and stepped wizard header
pub fn render_wizard_chrome(
    pixels: &mut [u8],
    stride: u32,
    w: u32,
    h: u32,
    current_step: usize,
    step_title: &str,
) {
    // 1. Root canvas fill (OLED Void)
    fill_rect_on_slice(pixels, stride, w, h, 0, 0, w as i32, h as i32, COLOR_BG);

    // 2. Center Card Frame (Double-bezel outer geometry)
    let card_w = 680i32;
    let card_h = 500i32;
    let card_x = (w as i32 - card_w) / 2;
    let card_y = (h as i32 - card_h) / 2;

    // Card outer border (1px)
    fill_rect_on_slice(pixels, stride, w, h, card_x - 1, card_y - 1, card_w + 2, card_h + 2, COLOR_BORDER);
    // Card background fill
    fill_rect_on_slice(pixels, stride, w, h, card_x, card_y, card_w, card_h, COLOR_CARD);

    // Top Brand Bar inside card
    draw_text_on_slice(pixels, stride, w, card_x + 24, card_y + 20, "MAHINA OS", COLOR_TEXT_PRIMARY);
    draw_text_on_slice(pixels, stride, w, card_x + 104, card_y + 20, "|  System Installer", COLOR_TEXT_MUTED);

    // Stepper header
    let steps = ["1. Welcome", "2. Target Disk", "3. Identity", "4. Install"];
    let mut step_x = card_x + card_w - 380;
    for (i, step) in steps.iter().enumerate() {
        let col = if i + 1 == current_step {
            COLOR_ACCENT
        } else if i + 1 < current_step {
            COLOR_SUCCESS
        } else {
            COLOR_TEXT_MUTED
        };
        draw_text_on_slice(pixels, stride, w, step_x, card_y + 20, step, col);
        step_x += 92;
    }

    // Hairline divider below header
    fill_rect_on_slice(pixels, stride, w, h, card_x, card_y + 48, card_w, 1, COLOR_BORDER);

    // Section Title
    draw_text_on_slice(pixels, stride, w, card_x + 24, card_y + 64, step_title, COLOR_TEXT_PRIMARY);
}
