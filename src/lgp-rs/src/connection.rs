use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::io::AsRawFd;
use crate::protocol::*;

pub struct LgpConnection {
    pub stream: UnixStream,
    pub caps_granted: u32,
    pub output_width: u32,
    pub output_height: u32,
}

impl LgpConnection {
    pub fn connect(caps_requested: u32) -> std::io::Result<Self> {
        let stream = UnixStream::connect(LGP_SOCKET_PATH)?;
        stream.set_nonblocking(false)?;

        let mut conn = LgpConnection {
            stream,
            caps_granted: 0,
            output_width: 0,
            output_height: 0,
        };

        conn.do_hello(caps_requested)?;
        conn.read_output_geometry()?;

        Ok(conn)
    }

    fn do_hello(&mut self, caps_requested: u32) -> std::io::Result<()> {
        let mut payload = vec![0u8; 8];
        write_u16_le(&mut payload, 0, LGP_VERSION_MAJOR);
        write_u16_le(&mut payload, 2, LGP_VERSION_MINOR);
        write_u32_le(&mut payload, 4, caps_requested);

        let msg = LgpMessage::new(LgpMessageType::Hello as u16, &payload);
        msg.send_to(&mut self.stream)?;

        let reply = LgpMessage::recv_from(&mut self.stream)?;
        if reply.msg_type != LgpMessageType::HelloReply as u16 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected HELLO_REPLY, got 0x{:04X}", reply.msg_type),
            ));
        }

        if reply.payload.len() >= 8 {
            self.caps_granted = read_u32_le(&reply.payload, 4);
        }

        Ok(())
    }

    fn read_output_geometry(&mut self) -> std::io::Result<()> {
        let msg = LgpMessage::recv_from(&mut self.stream)?;
        if msg.msg_type == LgpMessageType::OutputGeometry as u16 && msg.payload.len() >= 8 {
            self.output_width = read_u32_le(&msg.payload, 0);
            self.output_height = read_u32_le(&msg.payload, 4);
        }
        Ok(())
    }

    pub fn send(&mut self, msg: &LgpMessage) -> std::io::Result<()> {
        msg.send_to(&mut self.stream)
    }

    pub fn recv(&mut self) -> std::io::Result<LgpMessage> {
        LgpMessage::recv_from(&mut self.stream)
    }

    pub fn set_nonblocking(&mut self, nonblocking: bool) -> std::io::Result<()> {
        self.stream.set_nonblocking(nonblocking)
    }

    pub fn fd(&self) -> std::os::unix::io::RawFd {
        self.stream.as_raw_fd()
    }
}

impl std::os::unix::io::AsRawFd for LgpConnection {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.stream.as_raw_fd()
    }
}

impl Read for LgpConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for LgpConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}
