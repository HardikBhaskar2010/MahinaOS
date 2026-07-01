/*
 * Copyright (c) 2026 Hardik Bhaskar
 * Licensed under the MIT License.
 *
 * widgets.rs — Desktop widgets: real-time clock (from /proc/uptime + RTC),
 *              actual CPU/RAM monitor from /proc/stat and /proc/meminfo,
 *              and Luna Island notification island.
 */

use lunagui::canvas::{fill_rect_on_slice, draw_text_on_slice};

// ── Time reading from /proc/uptime ───────────────────────────────────────────

/// Read current wall time from /proc/rtc or fall back to uptime-based clock.
/// Returns (hour, minute, second).
pub fn read_realtime() -> (u8, u8, u8) {
    // Try /proc/driver/rtc (available in QEMU virtio)
    if let Ok(rtc) = std::fs::read_to_string("/proc/driver/rtc") {
        let mut h = 0u8; let mut m = 0u8; let mut s = 0u8;
        for line in rtc.lines() {
            if let Some(val) = line.strip_prefix("rtc_time\t: ") {
                let parts: Vec<&str> = val.split(':').collect();
                if parts.len() >= 3 {
                    h = parts[0].trim().parse().unwrap_or(0);
                    m = parts[1].trim().parse().unwrap_or(0);
                    s = parts[2].trim().parse().unwrap_or(0);
                }
            }
        }
        return (h, m, s);
    }

    // Fall back: derive from uptime (seconds since boot % 86400)
    let uptime_secs = read_uptime_secs();
    let secs_in_day = (uptime_secs as u64) % 86400;
    let h = (secs_in_day / 3600) as u8;
    let m = ((secs_in_day % 3600) / 60) as u8;
    let s = (secs_in_day % 60) as u8;
    (h, m, s)
}

pub fn read_uptime_secs() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(str::to_owned))
        .and_then(|s| s.parse::<f64>().ok())
        .map(|f| f as u64)
        .unwrap_or(0)
}

pub fn read_uptime_display() -> String {
    let secs = read_uptime_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}


// ── CPU usage from /proc/stat ────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct CpuSnapshot {
    user: u64, nice: u64, system: u64, idle: u64,
    iowait: u64, irq: u64, softirq: u64,
}

impl CpuSnapshot {
    fn total_active(&self) -> u64 {
        self.user + self.nice + self.system + self.irq + self.softirq
    }
    fn total(&self) -> u64 {
        self.total_active() + self.idle + self.iowait
    }
}

fn read_cpu_snapshot() -> Option<CpuSnapshot> {
    let stat = std::fs::read_to_string("/proc/stat").ok()?;
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let nums: Vec<u64> = line.split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 7 { return None; }
    Some(CpuSnapshot {
        user: nums[0], nice: nums[1], system: nums[2], idle: nums[3],
        iowait: nums[4], irq: nums[5], softirq: nums[6],
    })
}

// ── RAM usage from /proc/meminfo ─────────────────────────────────────────────

pub struct MemInfo {
    pub total_kb: u64,
    pub available_kb: u64,
}

impl MemInfo {
    pub fn used_percent(&self) -> u32 {
        if self.total_kb == 0 { return 0; }
        ((self.total_kb - self.available_kb) * 100 / self.total_kb) as u32
    }
    pub fn used_mb(&self) -> u64 {
        (self.total_kb - self.available_kb) / 1024
    }
    pub fn total_mb(&self) -> u64 {
        self.total_kb / 1024
    }
}

fn read_meminfo() -> MemInfo {
    let text = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut available = 0u64;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v.split_whitespace().next()
                .and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            available = v.split_whitespace().next()
                .and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    MemInfo { total_kb: total, available_kb: available }
}

// ── Luna Island ───────────────────────────────────────────────────────────────

/// Luna Island: macOS-style notification/status island at the top-center.
/// Shows: clock, active workspace indicator, CPU%, RAM bar.
const ISLAND_W: i32 = 320;
const ISLAND_H: i32 = 32;
const ISLAND_CORNER_R: i32 = 10; // for rendering approximation

pub struct LunaIsland {
    pub active_workspace: usize,
    pub workspace_count: usize,
    pub cpu_percent: u32,
    pub mem: MemInfo,
    pub clock: (u8, u8, u8),   // h, m, s
    prev_cpu: CpuSnapshot,
    refresh_counter: u32,
}

impl LunaIsland {
    pub fn new() -> Self {
        Self {
            active_workspace: 0,
            workspace_count: 6,
            cpu_percent: 0,
            mem: MemInfo { total_kb: 0, available_kb: 0 },
            clock: (0, 0, 0),
            prev_cpu: CpuSnapshot::default(),
            refresh_counter: 0,
        }
    }

    /// Call ~30fps. Updates stats every ~1s (30 ticks).
    pub fn tick(&mut self) {
        self.refresh_counter += 1;

        // Update clock every tick (cheap)
        self.clock = read_realtime();

        // Update CPU/RAM every 30 ticks (~1 second)
        if self.refresh_counter >= 30 {
            self.refresh_counter = 0;
            self.refresh_stats();
        }
    }

    fn refresh_stats(&mut self) {
        // CPU usage = delta active / delta total between snapshots
        if let Some(curr) = read_cpu_snapshot() {
            let dt = curr.total().saturating_sub(self.prev_cpu.total());
            let da = curr.total_active().saturating_sub(self.prev_cpu.total_active());
            self.cpu_percent = if dt > 0 { (da * 100 / dt) as u32 } else { 0 };
            self.prev_cpu = curr;
        }
        self.mem = read_meminfo();
    }

    /// Render the Luna Island pill into the overlay surface (centered at top).
    pub fn render(&self, pixels: &mut [u8], stride: u32, screen_w: u32, _screen_h: u32) {
        let sw = screen_w as i32;
        let ix = (sw - ISLAND_W) / 2;
        let iy = 0i32;

        // Island background — pill shape approximated as rect + rounded visual via color
        let bg = 0xE8101420u32;
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h, ix, iy, ISLAND_W, ISLAND_H, bg);

        // Bottom border accent glow
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
            ix, iy + ISLAND_H - 1, ISLAND_W, 1, 0x40E03E8Au32);

        let mut cx = ix + 10;

        // ── Workspace indicators ──────────────────────────────────────────
        for i in 0..self.workspace_count {
            let active = i == self.active_workspace;
            let dot_color = if active { 0xFFE03E8A } else { 0xFF444466 };
            let dot_size = if active { 8 } else { 5 };
            let dy = (ISLAND_H - dot_size) / 2;
            fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
                cx, iy + dy, dot_size, dot_size, dot_color);
            cx += dot_size + 4;
        }

        // Separator
        cx += 4;
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
            cx, iy + 8, 1, ISLAND_H - 16, 0xFF333355);
        cx += 8;

        // ── Clock (HH:MM:SS) ──────────────────────────────────────────────
        let clock_str = format!("{:02}:{:02}:{:02}", self.clock.0, self.clock.1, self.clock.2);
        draw_text_on_slice(pixels, stride, screen_w, cx, iy + 8, &clock_str, 0xFFEEEEF4);
        cx += clock_str.len() as i32 * 8 + 8;

        // Separator
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
            cx, iy + 8, 1, ISLAND_H - 16, 0xFF333355);
        cx += 8;

        // ── CPU bar ───────────────────────────────────────────────────────
        draw_text_on_slice(pixels, stride, screen_w, cx, iy + 8, "CPU", 0xFF9898AA);
        cx += 28;

        let cpu_bar_w = 40i32;
        let cpu_fill = (cpu_bar_w * self.cpu_percent as i32 / 100).min(cpu_bar_w);
        let cpu_color = if self.cpu_percent > 80 { 0xFF_E03E3E }
                        else if self.cpu_percent > 50 { 0xFF_E09A3E }
                        else { 0xFF_3EE09A };
        // Bar background
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
            cx, iy + 10, cpu_bar_w, 12, 0xFF_222238);
        // Bar fill
        if cpu_fill > 0 {
            fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
                cx, iy + 10, cpu_fill, 12, cpu_color);
        }
        // Percent text
        let cpu_str = format!("{:2}%", self.cpu_percent);
        draw_text_on_slice(pixels, stride, screen_w, cx + cpu_bar_w + 2, iy + 8, &cpu_str, 0xFF9898AA);
        cx += cpu_bar_w + 30;

        // Separator
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
            cx, iy + 8, 1, ISLAND_H - 16, 0xFF333355);
        cx += 8;

        // ── RAM bar ───────────────────────────────────────────────────────
        draw_text_on_slice(pixels, stride, screen_w, cx, iy + 8, "RAM", 0xFF9898AA);
        cx += 28;

        let ram_pct = self.mem.used_percent();
        let ram_bar_w = 40i32;
        let ram_fill = (ram_bar_w * ram_pct as i32 / 100).min(ram_bar_w);
        let ram_color = if ram_pct > 80 { 0xFF_E03E3E }
                        else if ram_pct > 60 { 0xFF_E09A3E }
                        else { 0xFF_6B7FD4 };
        fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
            cx, iy + 10, ram_bar_w, 12, 0xFF_222238);
        if ram_fill > 0 {
            fill_rect_on_slice(pixels, stride, screen_w, _screen_h,
                cx, iy + 10, ram_fill, 12, ram_color);
        }
        let ram_str = format!("{:4}M", self.mem.used_mb());
        draw_text_on_slice(pixels, stride, screen_w, cx + ram_bar_w + 2, iy + 8, &ram_str, 0xFF9898AA);
    }
}

// ── Desktop Widgets ───────────────────────────────────────────────────────────

pub struct DesktopWidgets {
    pub island: LunaIsland,
}

impl DesktopWidgets {
    pub fn new() -> Self {
        let mut island = LunaIsland::new();
        // Immediate first read
        island.refresh_stats();
        island.clock = read_realtime();
        Self { island }
    }

    pub fn tick(&mut self) {
        self.island.tick();
    }

    pub fn set_workspace(&mut self, active: usize, count: usize) {
        self.island.active_workspace = active;
        self.island.workspace_count = count;
    }

    pub fn render(&self, pixels: &mut [u8], stride: u32, sw: u32, sh: u32) {
        self.island.render(pixels, stride, sw, sh);
    }
}
