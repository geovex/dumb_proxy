use super::config_loader::Config;

pub async fn spawn(config: Config) {
    let mut joins = Vec::new();
    //http
    for (_k, v) in config.http {
        joins.push(tokio::spawn(async move {super::http::http(v.port).await}));
    }
    //socks4
    for (_k, v) in config.socks4 {
        joins.push(tokio::spawn(async move {super::socks4::socks4(v.port).await}));
    }
    //socks5
    for (_k, v) in config.socks5 {
        joins.push(tokio::spawn(async move {super::socks5::socks5(v.port).await}));
    }
    //tcppm
    for (_k, v) in config.tcppm {
        joins.push(tokio::spawn(async move {super::tcppm::tcppm(v.port, super::util::resolve_sockaddr(v.target).await.unwrap()).await}));
    }
    joins.shrink_to_fit();
    ::futures::future::join_all(joins).await;
}