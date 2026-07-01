use crate::protocol::*;

pub struct PointerEvent {
    pub x: f64,
    pub y: f64,
}

pub struct PointerButtonEvent {
    pub button: u32,
    pub pressed: bool,
}

pub struct KeyboardEvent {
    pub key: u32,
    pub modifiers: u32,
    pub pressed: bool,
}

pub fn parse_pointer_motion(payload: &[u8]) -> Option<PointerEvent> {
    if payload.len() < 8 { return None; }
    Some(PointerEvent {
        x: read_u32_le(payload, 0) as f64,
        y: read_u32_le(payload, 4) as f64,
    })
}

pub fn parse_pointer_button(payload: &[u8]) -> Option<PointerButtonEvent> {
    if payload.len() < 2 { return None; }
    Some(PointerButtonEvent {
        button: payload[0] as u32,
        pressed: payload[1] != 0,
    })
}

pub fn parse_keyboard_key(payload: &[u8]) -> Option<KeyboardEvent> {
    if payload.len() < 12 { return None; }
    Some(KeyboardEvent {
        key: read_u32_le(payload, 0),
        pressed: read_u32_le(payload, 4) != 0,
        modifiers: read_u32_le(payload, 8),
    })
}
