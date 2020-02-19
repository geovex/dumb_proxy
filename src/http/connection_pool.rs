use crate::util::resolve_sockaddr;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use tokio::io::Result;
use tokio::net::TcpStream;

pub struct SockRef<'cp> {
    sock: Option<TcpStream>,
    domain_port: String,
    pool: &'cp ConnectionPool,
}

impl SockRef<'_> {
    pub fn close(&mut self) {
        self.sock.take();
    }
}

impl Drop for SockRef<'_> {
    fn drop(&mut self) {
        if let Some(sock) = self.sock.take() {
            let dp = self.domain_port.clone();
            self.pool.connections.lock().unwrap().insert(dp, sock);
        }
    }
}

impl Deref for SockRef<'_> {
    type Target = TcpStream;
    fn deref(&self) -> &Self::Target {
        self.sock.as_ref().unwrap()
    }
}

impl DerefMut for SockRef<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.sock.as_mut().unwrap()
    }
}

pub struct ConnectionPool {
    connections: Mutex<HashMap<String, TcpStream>>,
}

impl ConnectionPool {
    pub fn new() -> ConnectionPool {
        ConnectionPool {
            connections: Mutex::new(HashMap::new()),
        }
    }
    pub async fn connect_or_reuse<'cp>(
        &'cp mut self,
        domain_port: String,
    ) -> Result<SockRef<'cp>> {
        let temp = self.connections.lock().unwrap().remove(&domain_port);
        let sock = if let Some(sock) = temp {
            sock
        } else {
            let sockaddr = resolve_sockaddr(domain_port.clone()).await?;
            TcpStream::connect(sockaddr).await?
        };
        Ok(SockRef {
            sock: Some(sock),
            domain_port,
            pool: self,
        })
    }
}
