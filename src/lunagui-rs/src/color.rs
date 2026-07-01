pub const COLOR_BG: u32 = 0xFF1E1E28;
pub const COLOR_RAISED: u32 = 0xFF262636;
pub const COLOR_HOVER: u32 = 0xFF2E2E42;
pub const COLOR_PRESSED: u32 = 0xFF181824;
pub const COLOR_ACCENT: u32 = 0xFF6B7FD4;
pub const COLOR_GLOW: u32 = 0xFF8A9FFF;
pub const COLOR_TEXT: u32 = 0xFFEEEEF4;
pub const COLOR_TEXT_SECONDARY: u32 = 0xFF9898AA;
pub const COLOR_TEXT_DISABLED: u32 = 0xFF555566;
pub const COLOR_BORDER: u32 = 0xFF303044;
pub const COLOR_BORDER_ACCENT: u32 = 0xFF4A5CA0;
pub const COLOR_TRANSPARENT: u32 = 0x00000000;

pub const COLOR_TOPBAR_BG: u32 = 0xEE0A0A14;
pub const COLOR_TOPBAR_BORDER: u32 = 0xFFE03E8A;
pub const COLOR_DOCK_BG: u32 = 0xB008090E;
pub const COLOR_DOCK_TILE: u32 = 0xCC151722;
pub const COLOR_DOCK_ACCENT: u32 = 0xFFE03E8A;
pub const COLOR_DOCK_TEXT: u32 = 0xFFD8D8E8;
pub const COLOR_DECO_BG: u32 = 0xE60F1018;
pub const COLOR_DECO_FOCUSED: u32 = 0xFFE03E8A;
pub const COLOR_DECO_BORDER: u32 = 0xFF5B3D78;

pub fn alpha_blend(src: u32, dst: u32) -> u32 {
    let a = (src >> 24) as u32;
    if a == 255 { return src; }
    if a == 0 { return dst | 0xFF000000; }
    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8) & 0xFF;
    let sb = src & 0xFF;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;
    let inv = 255 - a;
    let r = (sr * a + dr * inv) / 255;
    let g = (sg * a + dg * inv) / 255;
    let b = (sb * a + db * inv) / 255;
    0xFF000000 | (r << 16) | (g << 8) | b
}
