use k8s_openapi::api::core::v1::Namespace;
use kube::api::{Api, PostParams};
use kube::Client;
use serde_json::json;

pub async fn try_client() -> Option<Client> {
    match Client::try_default().await {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("No K8s client available: {e}");
            None
        }
    }
}

pub async fn ensure_namespace(
    client: &Client,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let ns: Namespace = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": { "name": name }
    }))?;
    match namespaces.create(&PostParams::default(), &ns).await {
        Ok(_) => Ok(()),
        Err(kube::Error::Api(e)) if e.code == 409 => Ok(()),
        Err(e) => Err(e.into()),
    }
}
