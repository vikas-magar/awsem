use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::Api;
use kube::Client;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

pub async fn port_forward(
    client: &Client,
    namespace: &str,
    pod_name: &str,
) -> Result<u16, Box<dyn std::error::Error>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

    // Find a free local port
    let local = TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = local.local_addr()?;
    let local_port = local_addr.port();
    drop(local);

    let addr: SocketAddr = ([127, 0, 0, 1], local_port).into();
    let pod_port: u16 = 9000;

    tracing::info!(
        "Port-forwarding localhost:{local_port} → {pod_name}:{pod_port}"
    );

    let pods_clone = pods.clone();
    let pod_name_str = pod_name.to_string();

    tokio::spawn(async move {
        let server = TcpListenerStream::new(
            TcpListener::bind(addr).await.unwrap(),
        );

        server
            .take_until(tokio::signal::ctrl_c())
            .try_for_each(|client_conn| async {
                let pods = pods_clone.clone();
                let pn = pod_name_str.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        forward_connection(&pods, &pn, pod_port, client_conn).await
                    {
                        tracing::error!("Port-forward error: {e}");
                    }
                });
                Ok(())
            })
            .await
            .ok();
    });

    Ok(local_port)
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: impl AsyncRead + AsyncWrite + Unpin,
) -> anyhow::Result<()> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder
        .take_stream(port)
        .ok_or_else(|| anyhow::anyhow!("port {port} not found in forwarder"))?;
    tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
    drop(upstream_conn);
    forwarder.join().await?;
    Ok(())
}
