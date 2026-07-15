use std::os::unix::io::AsRawFd;
use libc::{pollfd, poll, POLLIN};
use lgp::connection::LgpConnection;
use lgp::protocol::*;
use lgp::surface::LgpSurface;
use lgp::input::*;
use lunagui::canvas::{draw_text_on_slice, fill_rect_on_slice};

const COLOR_BG: u32 = 0xFF1E1E28;
const COLOR_TEXT: u32 = 0xFFEEEEF4;
const COLOR_TEXT_SEC: u32 = 0xFF9898AA;
const COLOR_ACCENT: u32 = 0xFF6B7FD4;
const COLOR_RAISED: u32 = 0xFF262636;
const COLOR_DEAD: u32 = 0xFF555566;

struct ProcInfo {
    pid: u32,
    name: String,
    state: String,
    cpu: f64,
    mem: u64,
}

struct TasksApp {
    procs: Vec<ProcInfo>,
    scroll: i32,
    selected: i32,
}

impl TasksApp {
    fn new() -> Self {
        let mut app = Self { procs: Vec::new(), scroll: 0, selected: 0 };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.procs.clear();
        if let Ok(dir) = std::fs::read_dir("/proc") {
            for entry in dir.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Ok(pid) = name_str.parse::<u32>() {
                    if let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", pid)) {
                        let mut pname = String::from("?");
                        let mut state = String::from("?");
                        let mut mem = 0u64;
                        for line in status.lines() {
                            if line.starts_with("Name:") {
                                pname = line[6..].trim().to_string();
                            }
                            if line.starts_with("State:") {
                                state = line[7..].trim().to_string();
                            }
                            if line.starts_with("VmRSS:") {
                                let parts: Vec<&str> = line[6..].split_whitespace().collect();
                                if !parts.is_empty() {
                                    mem = parts[0].parse().unwrap_or(0);
                                }
                            }
                        }
                        self.procs.push(ProcInfo {
                            pid,
                            name: pname,
                            state,
                            cpu: 0.0,
                            mem,
                        });
                    }
                }
            }
        }
        self.procs.sort_by_key(|p| p.pid);
        self.procs.truncate(80);
    }

    fn kill_selected(&self) -> std::io::Result<()> {
        if self.selected >= 0 && (self.selected as usize) < self.procs.len() {
            let pid = self.procs[self.selected as usize].pid;
            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
        }
        Ok(())
    }

    fn render(&self, pixels: &mut [u8], stride: u32, width: u32, height: u32) {
        fill_rect_on_slice(pixels, stride, width, height, 0, 0, width as i32, height as i32, COLOR_BG);

        draw_text_on_slice(pixels, stride, width, 8, 4, "PID   NAME                STATE   MEM", COLOR_ACCENT);
        fill_rect_on_slice(pixels, stride, width, height, 0, 22, width as i32, 1, COLOR_RAISED);

        let top_y = 24;
        let visible = (height as i32 - top_y) / 16;

        for i in 0..visible {
            let idx = self.scroll + i as i32;
            if idx < 0 || idx >= self.procs.len() as i32 { break; }
            let p = &self.procs[idx as usize];
            let py = top_y + i * 16;

            if idx == self.selected {
                fill_rect_on_slice(pixels, stride, width, height, 0, py, width as i32, 16, COLOR_ACCENT);
            }

            let state_color = if p.state.starts_with('R') { COLOR_TEXT }
                else if p.state.starts_with('S') { COLOR_TEXT_SEC }
                else { COLOR_DEAD };

            draw_text_on_slice(pixels, stride, width, 4, py,
                &format!("{:<5} {:<19} {:<7} {:>6}K", p.pid, p.name, p.state, p.mem), state_color);
        }

        draw_text_on_slice(pixels, stride, width, 8, height as i32 - 16,
            &format!("Processes: {}  [Up/Down: select, K: kill, ESC: quit]", self.procs.len()), COLOR_TEXT_SEC);
    }

    fn scroll_by(&mut self, delta: i32) {
        self.selected = (self.selected + delta).max(0).min(self.procs.len() as i32 - 1);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        let visible = 24;
        if self.selected >= self.scroll + visible {
            self.scroll = self.selected - visible + 1;
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut conn = LgpConnection::connect(LGP_CAP_CANVAS_SURFACE | LGP_CAP_KEYBOARD | LGP_CAP_POINTER)?;
    let width = 400;
    let height = 300;

    let mut surface = LgpSurface::create(
        &mut conn, LGP_SURFACE_CANVAS_SURFACE, 0, 0,
        width, height, LGP_LAYER_APPLICATION, true,
    )?;

    conn.set_nonblocking(true)?;

    let mut app = TasksApp::new();
    let mut refresh_counter = 0;

    loop {
        {
            let pixels = surface.pixels();
            app.render(pixels, width * 4, width, height);
        }
        surface.commit(&mut conn)?;

        let mut pfd = pollfd {
            fd: conn.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        let timeout = if refresh_counter % 20 == 0 { 16 } else { 100 };
        let ret = unsafe { poll(&mut pfd, 1, timeout) };
        if ret < 0 { continue; }

        refresh_counter += 1;
        if refresh_counter % 20 == 0 {
            app.refresh();
        }

        if (pfd.revents & POLLIN) != 0 {
            while let Ok(msg) = conn.recv() {
                match msg.msg_type {
                    t if t == LgpMessageType::KeyboardKey as u16 => {
                        if let Some(ev) = parse_keyboard_key(&msg.payload) {
                            if ev.pressed {
                                match ev.key {
                                    0x70052 => app.scroll_by(-1),  // Up
                                    0x70051 => app.scroll_by(1),   // Down
                                    0x7004A => app.scroll_by(-1),  // KP4
                                    0x7004E => app.scroll_by(1),   // KP6
                                    0x7004C => { let _ = app.kill_selected(); } // Delete
                                    0x70029 => return Ok(()),      // ESC
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
