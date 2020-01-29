use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};
use tokio::net::TcpStream;
use tokio;


pub async fn tcp_tranciever(mut src: TcpStream, mut dst: TcpStream) -> Result<()> {
    println!("tranciever");
    src.set_nodelay(true)?;
    dst.set_nodelay(true)?;
    let mut src_buf = [0u8; 2000];
    let mut dst_buf = [0u8; 2000];
    loop {
        tokio::select! {
            Ok(size) = src.read(&mut src_buf) => {
                if size == 0 {return Ok(())};
                dst.write_all(&src_buf[..size]).await?;
            } 
            Ok(size) = dst.read(&mut dst_buf) => {
                if size == 0 {return Ok(())};
                src.write_all(&dst_buf[..size]).await?;
            }
            else => {
                return Ok(());
            }
        }
    }
}