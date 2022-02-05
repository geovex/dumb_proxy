use crate::http::Http;
use crate::socks4::Socks4;
use crate::socks5::Socks5;

use super::config_loader::Config;

pub async fn spawn(config: Config) {
    let mut joins = Vec::new();
    //http
    for (k, v) in config.http {
        let http = Http::new(&k, &v);
        joins.push(tokio::spawn(async move {http.serve().await}));
    }
    //socks4
    for (k, v) in config.socks4 {
        let socks4 = Socks4::new(&k, &v);
        joins.push(tokio::spawn(async move {socks4.serve().await}));
    }
    //socks5
    for (k, v) in config.socks5 {
        let socks5 = Socks5::new(&k, &v);
        joins.push(tokio::spawn(async move {socks5.serve().await}));
    }
    //tcppm
    for (k, v) in config.tcppm {
        joins.push(tokio::spawn(async move {
            super::tcppm::tcppm(k, v.port, v.target).await
        }));
    }
    joins.shrink_to_fit();
    ::futures::future::join_all(joins).await;
}
