use std::io::{Read, Write};

pub const LGP_VERSION_MAJOR: u16 = 1;
pub const LGP_VERSION_MINOR: u16 = 0;
pub const LGP_HEADER_SIZE: usize = 6;
pub const LGP_SOCKET_PATH: &str = "/run/lgp/compositor.sock";

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LgpMessageType {
    Hello = 0x0001,
    HelloReply = 0x0002,
    CreateSurface = 0x0100,
    CreateSurfaceReply = 0x0101,
    DestroySurface = 0x0102,
    CommitBuffer = 0x0103,
    PointerMotion = 0x0110,
    PointerButton = 0x0111,
    KeyboardKey = 0x0112,
    PointerScroll = 0x0113,
    ClipboardSet = 0x0120,
    ClipboardGet = 0x0121,
    ClipboardData = 0x0122,
    WmSurfaceCreated = 0x0200,
    WmSurfaceDestroyed = 0x0201,
    WmSetSurfacePosition = 0x0202,
    WmSetFocus = 0x0203,
    WmSetState = 0x0204,
    WmGrabKey = 0x0205,
    OutputGeometry = 0x0300,
    Error = 0xFFFF,
}

impl LgpMessageType {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x0001 => Some(Self::Hello),
            0x0002 => Some(Self::HelloReply),
            0x0100 => Some(Self::CreateSurface),
            0x0101 => Some(Self::CreateSurfaceReply),
            0x0102 => Some(Self::DestroySurface),
            0x0103 => Some(Self::CommitBuffer),
            0x0110 => Some(Self::PointerMotion),
            0x0111 => Some(Self::PointerButton),
            0x0112 => Some(Self::KeyboardKey),
            0x0113 => Some(Self::PointerScroll),
            0x0120 => Some(Self::ClipboardSet),
            0x0121 => Some(Self::ClipboardGet),
            0x0122 => Some(Self::ClipboardData),
            0x0200 => Some(Self::WmSurfaceCreated),
            0x0201 => Some(Self::WmSurfaceDestroyed),
            0x0202 => Some(Self::WmSetSurfacePosition),
            0x0203 => Some(Self::WmSetFocus),
            0x0204 => Some(Self::WmSetState),
            0x0205 => Some(Self::WmGrabKey),
            0x0300 => Some(Self::OutputGeometry),
            0xFFFF => Some(Self::Error),
            _ => None,
        }
    }
}

pub const LGP_PIXEL_FORMAT_XRGB8888: u32 = 0x34325258;
pub const LGP_PIXEL_FORMAT_ARGB8888: u32 = 0x34325241;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LgpPixelFormat {
    Xrgb8888 = 0x34325258,
    Argb8888 = 0x34325241,
}

pub const LGP_SURFACE_SYSTEM_CHROME: u32 = 1;
pub const LGP_SURFACE_LUNA_ISLAND: u32 = 2;
pub const LGP_SURFACE_SHELL_SURFACE: u32 = 3;
pub const LGP_SURFACE_APPLICATION_WINDOW: u32 = 4;
pub const LGP_SURFACE_CANVAS_SURFACE: u32 = 5;
pub const LGP_SURFACE_OVERLAY_SURFACE: u32 = 6;

pub const LGP_SURFACE_STATUS_OK: u32 = 0;
pub const LGP_SURFACE_STATUS_INVALID: u32 = 1;
pub const LGP_SURFACE_STATUS_DENIED: u32 = 2;
pub const LGP_SURFACE_STATUS_NO_MEMORY: u32 = 3;
pub const LGP_SURFACE_STATUS_NOT_FOUND: u32 = 4;
pub const LGP_SURFACE_STATUS_BAD_BUFFER: u32 = 5;

pub const LGP_CAP_DMA_BUF: u32 = 1 << 0;
pub const LGP_CAP_CANVAS_SURFACE: u32 = 1 << 1;
pub const LGP_CAP_DIRECT_LGP: u32 = 1 << 2;
pub const LGP_CAP_LAYER_SHELL: u32 = 1 << 3;
pub const LGP_CAP_LUNA_ISLAND: u32 = 1 << 4;
pub const LGP_CAP_CURSOR_SHAPE: u32 = 1 << 5;
pub const LGP_CAP_CLIPBOARD: u32 = 1 << 6;
pub const LGP_CAP_WINDOW_MANAGER: u32 = 1 << 7;
pub const LGP_CAP_KEYBOARD: u32 = 1 << 8;
pub const LGP_CAP_POINTER: u32 = 1 << 9;

pub const LGP_WM_STATE_NORMAL: u32 = 0;
pub const LGP_WM_STATE_MAXIMIZED: u32 = 1;
pub const LGP_WM_STATE_MINIMIZED: u32 = 2;
pub const LGP_WM_STATE_FULLSCREEN: u32 = 3;
pub const LGP_WM_STATE_HIDDEN: u32 = 4;

pub const LGP_LAYER_WALLPAPER: u32 = 0;
pub const LGP_LAYER_APPLICATION: u32 = 100;
pub const LGP_LAYER_SHELL: u32 = 200;
pub const LGP_LAYER_OVERLAY: u32 = 300;
pub const LGP_LAYER_NOTIFICATION: u32 = 400;
pub const LGP_LAYER_LUNA_ISLAND: u32 = 500;
pub const LGP_LAYER_SYSTEM_MODAL: u32 = 600;
pub const LGP_LAYER_CURSOR: u32 = 700;

pub const LGP_MOD_SHIFT: u32 = 1;
pub const LGP_MOD_CTRL: u32 = 2;
pub const LGP_MOD_ALT: u32 = 4;
pub const LGP_MOD_SUPER: u32 = 8;
pub const LGP_MOD_CAPS: u32 = 16;

pub struct LgpMessage {
    pub msg_type: u16,
    pub payload: Vec<u8>,
}

impl LgpMessage {
    pub fn new(msg_type: u16, payload: &[u8]) -> Self {
        Self {
            msg_type,
            payload: payload.to_vec(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let total_len = LGP_HEADER_SIZE + self.payload.len();
        let mut buf = Vec::with_capacity(total_len);
        buf.extend_from_slice(&self.msg_type.to_le_bytes());
        buf.extend_from_slice(&(total_len as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < LGP_HEADER_SIZE {
            return None;
        }
        let msg_type = u16::from_le_bytes([data[0], data[1]]);
        let total_len = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
        if data.len() < total_len {
            return None;
        }
        let payload = data[LGP_HEADER_SIZE..total_len].to_vec();
        Some(Self { msg_type, payload })
    }

    pub fn send_to(&self, stream: &mut impl Write) -> std::io::Result<()> {
        let encoded = self.encode();
        stream.write_all(&encoded)
    }

    pub fn recv_from(stream: &mut impl Read) -> std::io::Result<Self> {
        let mut header = [0u8; LGP_HEADER_SIZE];
        stream.read_exact(&mut header)?;
        let msg_type = u16::from_le_bytes([header[0], header[1]]);
        let total_len = u32::from_le_bytes([header[2], header[3], header[4], header[5]]) as usize;
        let payload_len = total_len.saturating_sub(LGP_HEADER_SIZE);
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            stream.read_exact(&mut payload)?;
        }
        Ok(Self { msg_type, payload })
    }
}

pub fn write_u32_le(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

pub fn write_i32_le(buf: &mut [u8], offset: usize, val: i32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

pub fn write_u16_le(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

pub fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}

pub fn read_i32_le(buf: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}
