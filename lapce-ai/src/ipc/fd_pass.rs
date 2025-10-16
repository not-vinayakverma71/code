/// Unix domain socket file descriptor passing via SCM_RIGHTS
/// Used to transfer eventfd doorbells from server to client

use anyhow::{Result, bail};
use std::os::unix::io::RawFd;
use std::os::unix::net::UnixStream;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// Send file descriptors over Unix domain socket using SCM_RIGHTS
#[cfg(unix)]
pub fn send_fds(sock: &UnixStream, fds: &[RawFd], data: &[u8]) -> Result<()> {
    use libc::{msghdr, iovec, cmsghdr, sendmsg, CMSG_DATA, CMSG_SPACE, CMSG_LEN, SOL_SOCKET, SCM_RIGHTS};
    use std::mem;
    
    let sock_fd = sock.as_raw_fd();
    
    // Prepare iovec for data
    let mut iov = iovec {
        iov_base: data.as_ptr() as *mut libc::c_void,
        iov_len: data.len(),
    };
    
    // Prepare control message buffer for file descriptors
    let cmsg_space = unsafe { CMSG_SPACE((fds.len() * mem::size_of::<RawFd>()) as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    
    let mut msg: msghdr = unsafe { mem::zeroed() };
    msg.msg_iov = &mut iov as *mut iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as u32;
    
    // Fill in control message
    let cmsg = unsafe { &mut *(msg.msg_control as *mut cmsghdr) };
    cmsg.cmsg_len = unsafe { CMSG_LEN((fds.len() * mem::size_of::<RawFd>()) as u32) } as _;
    cmsg.cmsg_level = SOL_SOCKET;
    cmsg.cmsg_type = SCM_RIGHTS;
    
    // Copy file descriptors into control message
    unsafe {
        let cmsg_data = CMSG_DATA(cmsg);
        std::ptr::copy_nonoverlapping(
            fds.as_ptr() as *const u8,
            cmsg_data,
            fds.len() * mem::size_of::<RawFd>(),
        );
    }
    
    let ret = unsafe { sendmsg(sock_fd, &msg, 0) };
    
    if ret < 0 {
        bail!("Failed to send fds: {}", std::io::Error::last_os_error());
    }
    
    eprintln!("[FD_PASS] Sent {} fds with {} bytes data", fds.len(), data.len());
    Ok(())
}

/// Receive file descriptors over Unix domain socket using SCM_RIGHTS
#[cfg(unix)]
pub fn recv_fds(sock: &UnixStream, max_fds: usize) -> Result<(Vec<RawFd>, Vec<u8>)> {
    use libc::{msghdr, iovec, cmsghdr, recvmsg, CMSG_DATA, CMSG_SPACE, SOL_SOCKET, SCM_RIGHTS};
    use std::mem;
    
    let sock_fd = sock.as_raw_fd();
    
    // Buffer for data
    let mut data_buf = vec![0u8; 4096];
    let mut iov = iovec {
        iov_base: data_buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: data_buf.len(),
    };
    
    // Buffer for control message
    let cmsg_space = unsafe { CMSG_SPACE((max_fds * mem::size_of::<RawFd>()) as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    
    let mut msg: msghdr = unsafe { mem::zeroed() };
    msg.msg_iov = &mut iov as *mut iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as u32;
    
    let ret = unsafe { recvmsg(sock_fd, &mut msg, 0) };
    
    if ret < 0 {
        bail!("Failed to recv fds: {}", std::io::Error::last_os_error());
    }
    
    let data_len = ret as usize;
    data_buf.truncate(data_len);
    
    // Extract file descriptors from control message
    let mut fds = Vec::new();
    
    unsafe {
        let cmsg = msg.msg_control as *const cmsghdr;
        if !cmsg.is_null() && (*cmsg).cmsg_level == SOL_SOCKET && (*cmsg).cmsg_type == SCM_RIGHTS {
            let cmsg_data = CMSG_DATA(cmsg);
            let fd_count = ((*cmsg).cmsg_len as usize - mem::size_of::<cmsghdr>()) / mem::size_of::<RawFd>();
            
            for i in 0..fd_count {
                let fd_ptr = cmsg_data.add(i * mem::size_of::<RawFd>()) as *const RawFd;
                fds.push(*fd_ptr);
            }
        }
    }
    
    eprintln!("[FD_PASS] Received {} fds with {} bytes data", fds.len(), data_len);
    Ok((fds, data_buf))
}

#[cfg(not(unix))]
pub fn send_fds(_sock: &std::net::TcpStream, _fds: &[RawFd], _data: &[u8]) -> Result<()> {
    bail!("FD passing only supported on Unix");
}

#[cfg(not(unix))]
pub fn recv_fds(_sock: &std::net::TcpStream, _max_fds: usize) -> Result<(Vec<RawFd>, Vec<u8>)> {
    bail!("FD passing only supported on Unix");
}
