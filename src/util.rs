use tokio;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Result};

pub async fn transceiver<S, D>(src: &mut S, dst: &mut D) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
    D: AsyncRead + AsyncWrite + Unpin,
{
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

#[cfg(test)]
mod test {
    #[test]
    #[should_panic]
    fn invalid_resolve() {
        use std::net::ToSocketAddrs as _;
        "127.0.0.1:80:70".to_socket_addrs().unwrap();
    }
}
