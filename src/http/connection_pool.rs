use crate::util::resolve_sockaddr;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use tokio::io::Result;
use tokio::net::TcpStream;
use lru_cache::LruCache;

const LRU_CACHE_SIZE: usize = 10;

pub struct SockRef<'cp> {
    sock: Option<TcpStream>,
    domain_port: String,
    pool: &'cp ConnectionPool,
}

impl SockRef<'_> {
    // pub fn close(&mut self) {
    //     self.sock.take();
    // }
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
    connections: Mutex<LruCache<String, TcpStream>>,
}

impl ConnectionPool {
    pub fn new() -> ConnectionPool {
        ConnectionPool {
            connections: Mutex::new(LruCache::new(LRU_CACHE_SIZE)),
        }
    }
    pub async fn connect_or_reuse<'cp>(
        &'cp mut self,
        domain_port: &String,
    ) -> Result<SockRef<'cp>> {
        let temp = self.connections.lock().unwrap().remove(domain_port);
        let sock = if let Some(sock) = temp {
            sock
        } else {
            let sockaddr = resolve_sockaddr(domain_port.clone()).await?;
            let sock = TcpStream::connect(sockaddr).await?;
            sock.set_nodelay(true)?;
            sock
        };
        Ok(SockRef {
            sock: Some(sock),
            domain_port: domain_port.clone(),
            pool: self,
        })
    }
}
