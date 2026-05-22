use k8s_openapi::api::core::v1::Pod;
use kube::api::Api;
use kube::Client;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;

pub async fn port_forward(client: &Client, namespace: &str, pod_name: &str) -> Result<u16, Box<dyn std::error::Error>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let local = TcpListener::bind("127.0.0.1:0").await?;
    let port = local.local_addr()?.port();
    let pod_port: u16 = 9000;
    let pods = pods.clone();
    let pn = pod_name.to_string();

    tokio::spawn(async move {
        loop {
            let (conn, _) = match local.accept().await {
                Ok(c) => c, Err(e) => { tracing::error!("pf accept: {e}"); continue; }
            };
            let p = pods.clone();
            let name = pn.clone();
            tokio::spawn(async move {
                if let Err(e) = forward(&p, &name, pod_port, conn).await {
                    tracing::error!("pf conn: {e}");
                }
            });
        }
    });

    Ok(port)
}

async fn forward(pods: &Api<Pod>, pod_name: &str, port: u16, mut client: impl AsyncRead + AsyncWrite + Unpin) -> anyhow::Result<()> {
    let mut fwd = pods.portforward(pod_name, &[port]).await?;
    let mut upstream = fwd.take_stream(port).ok_or_else(|| anyhow::anyhow!("port {port} not found"))?;
    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
    drop(upstream);
    drop(fwd);
    Ok(())
}
