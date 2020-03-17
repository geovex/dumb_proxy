use super::config_loader::Config;

pub async fn spawn(config: Config) {
    let mut joins = Vec::new();
    //http
    for (k, v) in config.http {
        joins.push(tokio::spawn(async move {super::http::http(k, v.port).await}));
    }
    //socks4
    for (k, v) in config.socks4 {
        joins.push(tokio::spawn(async move {super::socks4::socks4(k, v.port).await}));
    }
    //socks5
    for (k, v) in config.socks5 {
        joins.push(tokio::spawn(async move {super::socks5::socks5(k, v.port).await}));
    }
    //tcppm
    for (k, v) in config.tcppm {
        joins.push(tokio::spawn(async move {super::tcppm::tcppm(k, v.port, super::util::resolve_sockaddr(v.target).await.unwrap()).await}));
    }
    joins.shrink_to_fit();
    ::futures::future::join_all(joins).await;
}