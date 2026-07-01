use crate::connection::LgpConnection;
use crate::protocol::*;

pub struct WmSurfaceInfo {
    pub surface_id: u32,
    pub surface_type: u32,
    pub width: u32,
    pub height: u32,
}

pub struct WmState;

impl WmState {
    pub fn parse_surface_created(payload: &[u8]) -> Option<WmSurfaceInfo> {
        if payload.len() < 16 { return None; }
        Some(WmSurfaceInfo {
            surface_id: read_u32_le(payload, 0),
            surface_type: read_u32_le(payload, 4),
            width: read_u32_le(payload, 8),
            height: read_u32_le(payload, 12),
        })
    }

    pub fn parse_surface_destroyed(payload: &[u8]) -> Option<u32> {
        if payload.len() < 4 { return None; }
        Some(read_u32_le(payload, 0))
    }

    pub fn set_focus(conn: &mut LgpConnection, surface_id: u32) -> std::io::Result<()> {
        let mut payload = vec![0u8; 4];
        write_u32_le(&mut payload, 0, surface_id);
        let msg = LgpMessage::new(LgpMessageType::WmSetFocus as u16, &payload);
        msg.send_to(&mut conn.stream)
    }

    pub fn set_state(conn: &mut LgpConnection, surface_id: u32, state: u32) -> std::io::Result<()> {
        let mut payload = vec![0u8; 8];
        write_u32_le(&mut payload, 0, surface_id);
        write_u32_le(&mut payload, 4, state);
        let msg = LgpMessage::new(LgpMessageType::WmSetState as u16, &payload);
        msg.send_to(&mut conn.stream)
    }

    pub fn grab_key(conn: &mut LgpConnection, key: u32, modifiers: u32) -> std::io::Result<()> {
        let mut payload = vec![0u8; 8];
        write_u32_le(&mut payload, 0, key);
        write_u32_le(&mut payload, 4, modifiers);
        let msg = LgpMessage::new(LgpMessageType::WmGrabKey as u16, &payload);
        msg.send_to(&mut conn.stream)
    }
}
