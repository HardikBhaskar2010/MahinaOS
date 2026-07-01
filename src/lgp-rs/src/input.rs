use crate::protocol::*;

pub struct PointerEvent {
    pub surface_id: u32,
    pub x: f64,
    pub y: f64,
}

pub struct PointerButtonEvent {
    pub surface_id: u32,
    pub x: f64,
    pub y: f64,
    pub button: u32,
    pub pressed: bool,
}

pub struct KeyboardEvent {
    pub surface_id: u32,
    pub key: u32,
    pub modifiers: u32,
    pub pressed: bool,
}

pub fn parse_pointer_motion(payload: &[u8]) -> Option<PointerEvent> {
    if payload.len() < 20 { return None; }
    Some(PointerEvent {
        surface_id: read_u32_le(payload, 0),
        x: f64::from_bits(read_u32_le(payload, 4) as u64),
        y: f64::from_bits(read_u32_le(payload, 12) as u64),
    })
}

pub fn parse_pointer_button(payload: &[u8]) -> Option<PointerButtonEvent> {
    if payload.len() < 24 { return None; }
    Some(PointerButtonEvent {
        surface_id: read_u32_le(payload, 0),
        x: f64::from_bits(read_u32_le(payload, 4) as u64),
        y: f64::from_bits(read_u32_le(payload, 12) as u64),
        button: read_u32_le(payload, 20),
        pressed: payload[24] != 0,
    })
}

pub fn parse_keyboard_key(payload: &[u8]) -> Option<KeyboardEvent> {
    if payload.len() < 13 { return None; }
    Some(KeyboardEvent {
        surface_id: read_u32_le(payload, 0),
        key: read_u32_le(payload, 4),
        modifiers: read_u32_le(payload, 8),
        pressed: payload[12] != 0,
    })
}
