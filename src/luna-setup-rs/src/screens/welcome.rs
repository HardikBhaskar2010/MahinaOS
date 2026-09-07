use super::{render_wizard_chrome, Screen, WizardState, COLOR_BORDER, COLOR_CARD_INNER, COLOR_SUCCESS, COLOR_TEXT_MUTED, COLOR_TEXT_SEC};
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};
use lunagui::Button;
use lunagui::Widget;

pub struct WelcomeScreen {
    next_btn: Button,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        Self {
            next_btn: Button::accent(0, 0, 160, 36, "Begin Setup ->"),
        }
    }
}

impl Screen for WelcomeScreen {
    fn render(&mut self, pixels: &mut [u8], stride: u32, w: u32, h: u32) {
        render_wizard_chrome(pixels, stride, w, h, 1, "Welcome to MahinaOS");

        let card_w = 680i32;
        let card_h = 500i32;
        let card_x = (w as i32 - card_w) / 2;
        let card_y = (h as i32 - card_h) / 2;

        let content_x = card_x + 24;
        let mut y = card_y + 96;

        draw_text_on_slice(pixels, stride, w, content_x, y, "The machine you own. The partner you trust.", COLOR_TEXT_SEC);
        y += 28;

        // Information banner card (Double-bezel inner surface)
        let banner_w = card_w - 48;
        let banner_h = 240;
        fill_rect_on_slice(pixels, stride, w, h, content_x - 1, y - 1, banner_w + 2, banner_h + 2, COLOR_BORDER);
        fill_rect_on_slice(pixels, stride, w, h, content_x, y, banner_w, banner_h, COLOR_CARD_INNER);

        let bx = content_x + 16;
        let mut by = y + 16;

        draw_text_on_slice(pixels, stride, w, bx, by, "FOUNDATION: Trustworthy Machine", COLOR_SUCCESS);
        by += 18;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Linux 6.6 LTS kernel with built-in virtio, DRM/KMS, and Btrfs", COLOR_TEXT_MUTED);
        by += 16;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Fast, deterministic UEFI boot via Limine Bootloader", COLOR_TEXT_MUTED);
        by += 16;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Static musl luna-init (PID 1) process supervisor with zero NSS bloat", COLOR_TEXT_MUTED);
        by += 26;

        draw_text_on_slice(pixels, stride, w, bx, by, "PLATFORM: Native Linux Environment", COLOR_SUCCESS);
        by += 18;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Clean-slate LGP compositor over direct DRM/KMS dumb buffers", COLOR_TEXT_MUTED);
        by += 16;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Generational Btrfs state management with instant A/B rollback", COLOR_TEXT_MUTED);
        by += 16;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Cryptographically verified software distribution via lpkg", COLOR_TEXT_MUTED);
        by += 26;

        draw_text_on_slice(pixels, stride, w, bx, by, "INTELLIGENCE: Permissioned System Partner", COLOR_SUCCESS);
        by += 18;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Local luna-ai-d intelligence bounded by fine-grained capabilities", COLOR_TEXT_MUTED);
        by += 16;
        draw_text_on_slice(pixels, stride, w, bx, by, "* Self-modification isolated in verified candidate generations", COLOR_TEXT_MUTED);

        // Footer button
        self.next_btn.set_pos(card_x + card_w - 24 - 160, card_y + card_h - 24 - 36);
        self.next_btn.render(pixels, stride, w, h);
    }

    fn on_click(&mut self, x: i32, y: i32) -> Option<WizardState> {
        if self.next_btn.hit_test(x, y) {
            return Some(WizardState::DiskSelect);
        }
        None
    }

    fn on_key(&mut self, key: u32, pressed: bool) {
        if pressed && (key == 0x70028 || key == 0x28) {
            // Enter key advances
        }
    }
}
