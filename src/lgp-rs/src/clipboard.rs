use crate::connection::LgpConnection;
use crate::protocol::*;

pub struct Clipboard;

impl Clipboard {
    pub fn set_text(conn: &mut LgpConnection, text: &str) -> std::io::Result<()> {
        let bytes = text.as_bytes();
        let mut payload = Vec::with_capacity(4 + bytes.len());
        write_u32_le(&mut payload, 0, bytes.len() as u32);
        payload.extend_from_slice(bytes);
        let msg = LgpMessage::new(LgpMessageType::ClipboardSet as u16, &payload);
        msg.send_to(&mut conn.stream)
    }

    pub fn request_text(conn: &mut LgpConnection) -> std::io::Result<()> {
        let payload = vec![0u8; 0];
        let msg = LgpMessage::new(LgpMessageType::ClipboardGet as u16, &payload);
        msg.send_to(&mut conn.stream)
    }

    pub fn parse_data(payload: &[u8]) -> Option<String> {
        if payload.len() < 4 { return None; }
        let len = read_u32_le(payload, 0) as usize;
        if 4 + len > payload.len() { return None; }
        String::from_utf8(payload[4..4 + len].to_vec()).ok()
    }
}
