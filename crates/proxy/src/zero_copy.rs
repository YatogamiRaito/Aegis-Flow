use std::io;
use tokio::net::TcpStream;

#[cfg(target_os = "linux")]
pub fn apply_tcp_cork(stream: &TcpStream) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let fd = stream.as_raw_fd();
    unsafe {
        let optval: libc::c_int = 1;
        if libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_CORK,
            &optval as *const _ as *const libc::c_void,
            std::mem::size_of_val(&optval) as libc::socklen_t,
        ) != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn apply_tcp_cork(_stream: &TcpStream) -> io::Result<()> {
    // macOS doesn't support TCP_CORK directly in standard way (it has TCP_NOPUSH)
    Ok(())
}

pub fn apply_tcp_nodelay(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_tcp_cork_nodelay() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move {
            let stream = TcpStream::connect(addr).await.unwrap();
            apply_tcp_nodelay(&stream).unwrap();
            apply_tcp_cork(&stream).unwrap();
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        apply_tcp_nodelay(&server_stream).unwrap();
        apply_tcp_cork(&server_stream).unwrap();

        client_task.await.unwrap();
    }
}
