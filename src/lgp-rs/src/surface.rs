use std::os::unix::io::RawFd;
use crate::connection::LgpConnection;
use crate::protocol::*;

pub struct LgpSurface {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub layer: u32,
    pub argb: bool,
    pub buffer_fd: RawFd,
    pub buffer_size: usize,
    pub buffer_map: *mut u8,
}

impl LgpSurface {
    pub fn create(
        conn: &mut LgpConnection,
        surface_type: u32,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        layer: u32,
        argb: bool,
    ) -> std::io::Result<Self> {
        let mut payload = vec![0u8; 24];
        write_u32_le(&mut payload, 0, surface_type);
        write_i32_le(&mut payload, 4, x);
        write_i32_le(&mut payload, 8, y);
        write_u32_le(&mut payload, 12, width);
        write_u32_le(&mut payload, 16, height);
        write_u32_le(&mut payload, 20, layer);

        let msg = LgpMessage::new(LgpMessageType::CreateSurface as u16, &payload);
        msg.send_to(&mut conn.stream)?;

        let reply = LgpMessage::recv_from(&mut conn.stream)?;
        if reply.msg_type != LgpMessageType::CreateSurfaceReply as u16 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected CREATE_SURFACE_REPLY, got 0x{:04X}", reply.msg_type),
            ));
        }

        if reply.payload.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "CREATE_SURFACE_REPLY payload too short",
            ));
        }

        let status = read_u32_le(&reply.payload, 0);
        let surface_id = read_u32_le(&reply.payload, 4);

        if status != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("CREATE_SURFACE rejected with status {}", status),
            ));
        }

        let buffer_size = (width as usize) * (height as usize) * 4;
        let buffer_fd = create_memfd(buffer_size)?;
        let buffer_map = mmap_buffer(buffer_fd, buffer_size)?;

        Ok(Self {
            id: surface_id,
            width,
            height,
            layer,
            argb,
            buffer_fd,
            buffer_size,
            buffer_map,
        })
    }

    pub fn destroy(&mut self, conn: &mut LgpConnection) -> std::io::Result<()> {
        let mut payload = vec![0u8; 4];
        write_u32_le(&mut payload, 0, self.id);
        let msg = LgpMessage::new(LgpMessageType::DestroySurface as u16, &payload);
        msg.send_to(&mut conn.stream)?;
        self.unmap_buffer();
        Ok(())
    }

    pub fn commit(&self, conn: &mut LgpConnection) -> std::io::Result<()> {
        let pixel_fmt = if self.argb {
            LGP_PIXEL_FORMAT_ARGB8888
        } else {
            LGP_PIXEL_FORMAT_XRGB8888
        };

        let mut payload = vec![0u8; 24];
        write_u32_le(&mut payload, 0, self.id);
        write_u32_le(&mut payload, 4, self.width);
        write_u32_le(&mut payload, 8, self.height);
        write_u32_le(&mut payload, 12, self.width * 4);
        write_u32_le(&mut payload, 16, pixel_fmt);
        write_u32_le(&mut payload, 20, self.buffer_size as u32);

        let total_len = LGP_HEADER_SIZE + payload.len();
        let mut buf = Vec::with_capacity(total_len);
        buf.extend_from_slice(&(LgpMessageType::CommitBuffer as u16).to_le_bytes());
        buf.extend_from_slice(&(total_len as u32).to_le_bytes());
        buf.extend_from_slice(&payload);

        send_with_fd(&mut conn.stream, &buf, self.buffer_fd)?;
        Ok(())
    }

    pub fn pixels(&self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.buffer_map, self.buffer_size) }
    }

    fn unmap_buffer(&mut self) {
        if !self.buffer_map.is_null() {
            unsafe {
                libc::munmap(self.buffer_map as *mut libc::c_void, self.buffer_size);
            }
            self.buffer_map = std::ptr::null_mut();
        }
    }

    pub fn set_position(&self, conn: &mut LgpConnection, x: i32, y: i32) -> std::io::Result<()> {
        let mut payload = vec![0u8; 12];
        write_u32_le(&mut payload, 0, self.id);
        write_i32_le(&mut payload, 4, x);
        write_i32_le(&mut payload, 8, y);
        let msg = LgpMessage::new(LgpMessageType::WmSetSurfacePosition as u16, &payload);
        msg.send_to(&mut conn.stream)
    }
}

impl Drop for LgpSurface {
    fn drop(&mut self) {
        if !self.buffer_map.is_null() {
            unsafe {
                libc::munmap(self.buffer_map as *mut libc::c_void, self.buffer_size);
            }
        }
        if self.buffer_fd >= 0 {
            unsafe {
                libc::close(self.buffer_fd);
            }
        }
    }
}

fn create_memfd(size: usize) -> std::io::Result<RawFd> {
    let fd = unsafe {
        libc::memfd_create(
            "lunagui_rs\0".as_ptr() as *const libc::c_char,
            libc::MFD_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
        unsafe { libc::close(fd) };
        return Err(std::io::Error::last_os_error());
    }
    Ok(fd)
}

fn mmap_buffer(fd: RawFd, size: usize) -> std::io::Result<*mut u8> {
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        unsafe { libc::close(fd) };
        return Err(std::io::Error::last_os_error());
    }
    Ok(ptr as *mut u8)
}

fn send_with_fd(stream: &mut impl std::os::unix::io::AsRawFd, data: &[u8], fd: RawFd) -> std::io::Result<()> {
use std::os::unix::io::{AsRawFd, RawFd};


    let iov = libc::iovec {
        iov_base: data.as_ptr() as *mut libc::c_void,
        iov_len: data.len(),
    };

    let mut cmsg_buf = [0u8; 32];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &iov as *const libc::iovec as *mut libc::iovec;
    msg.msg_iovlen = 1;

    let fd_size = std::mem::size_of::<RawFd>();
    let cmsg = unsafe {
        let hdr = &mut *(&mut cmsg_buf as *mut [u8; 32] as *mut libc::cmsghdr);
        hdr.cmsg_len = libc::CMSG_LEN(fd_size as _) as _;
        hdr.cmsg_level = libc::SOL_SOCKET;
        hdr.cmsg_type = libc::SCM_RIGHTS;
        std::ptr::write(
            libc::CMSG_DATA(hdr) as *mut RawFd,
            fd,
        );
        hdr
    };

    msg.msg_control = cmsg as *const libc::cmsghdr as *mut libc::c_void;
    msg.msg_controllen = unsafe { libc::CMSG_SPACE(fd_size as _) as _ };

    let ret = unsafe { libc::sendmsg(stream.as_raw_fd(), &msg, 0) };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
