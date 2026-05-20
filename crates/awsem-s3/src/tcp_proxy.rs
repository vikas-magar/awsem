use tokio::net::TcpListener;
use tokio::io::copy_bidirectional;
use std::net::SocketAddr;

pub async fn start_tcp_proxy(local_port: u16, upstream_addr: SocketAddr) {
    let local = TcpListener::bind(format!("127.0.0.1:{local_port}"))
        .await
        .expect("Failed to bind S3 proxy port");

    tracing::info!("S3 TCP proxy listening on 127.0.0.1:{local_port} → {upstream_addr}");

    loop {
        let (mut client, _) = match local.accept().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("TCP proxy accept error: {e}");
                continue;
            }
        };

        let upstream = match tokio::net::TcpStream::connect(upstream_addr).await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("TCP proxy connect error: {e}");
                continue;
            }
        };

        tokio::spawn(async move {
            let (mut cr, mut cw) = client.split();
            let (mut ur, mut uw) = upstream.split();
            let c2u = copy_bidirectional(&mut cr, &mut uw);
            let u2c = copy_bidirectional(&mut ur, &mut cw);
            tokio::join!(c2u, u2c);
        });
    }
}
