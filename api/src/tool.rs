#[cfg(feature = "blocking")]
pub fn ping(host: &str, port: u16) -> std::io::Result<()> {
    use std::{
        io::{Read, Write},
        net::TcpStream,
    };
    let mut socket = TcpStream::connect((host, port))?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    socket.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
    socket.write_all(b"GET / HTTP/1.0\r\n\r\n")?;
    let mut buffer = [0; 1];
    socket.read_exact(&mut buffer)?;
    Ok(())
}

#[cfg(feature = "async")]
pub async fn ping_async(host: &str, port: u16) -> tokio::io::Result<()> {
    use std::time::Duration;
    
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        time::timeout,
    };
    let mut socket = timeout(Duration::from_secs(2), TcpStream::connect((host, port))).await??;
    timeout(
        Duration::from_secs(2),
        socket.write_all(b"GET / HTTP/1.0\r\n\r\n"),
    )
    .await??;
    let mut buffer = [0; 1];
    timeout(Duration::from_secs(2), socket.read_exact(&mut buffer)).await??;
    Ok(())
}
