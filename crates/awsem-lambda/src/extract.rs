pub fn save_code_zip(data_dir: &Option<String>, name: &str, zip_data: &[u8]) {
    let dir = crate::name::data_dir_fn(data_dir, name);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::error!("Failed to create dir {dir}: {e}");
        return;
    }
    let zip_path = format!("{dir}/code.zip");
    if let Err(e) = std::fs::write(&zip_path, zip_data) {
        tracing::error!("Failed to write {zip_path}: {e}");
        return;
    }
    if let Ok(file) = std::fs::File::open(&zip_path) {
        extract_zip(file, &dir);
    }
}

use k8s_openapi::api::core::v1::Secret;
use kube::api::{DeleteParams, PostParams};

pub async fn create_secret(
    client: &kube::Client,
    ns: &str,
    name: &str,
    key: &str,
    data: &str,
) -> Result<(), String> {
    let api: kube::Api<Secret> = kube::Api::namespaced(client.clone(), ns);
    let secret: Secret = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Secret",
        "metadata": { "name": name, "namespace": ns },
        "stringData": { key: data },
    }))
    .map_err(|e| e.to_string())?;
    api.create(&PostParams::default(), &secret)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_secret(client: &kube::Client, ns: &str, name: &str) {
    let api: kube::Api<Secret> = kube::Api::namespaced(client.clone(), ns);
    if let Err(e) = api.delete(name, &DeleteParams::default()).await {
        tracing::warn!("Failed to delete Secret {name} in {ns}: {e}");
    }
}

fn extract_zip(file: std::fs::File, dest: &str) {
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to open zip: {e}");
            return;
        }
    };
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Zip entry {i}: {e}");
                continue;
            }
        };
        let out_path = match entry.enclosed_name() {
            Some(p) => p.to_owned(),
            None => continue,
        };
        let full_path = std::path::Path::new(dest).join(&out_path);
        if entry.is_dir() {
            if let Err(e) = std::fs::create_dir_all(&full_path) {
                tracing::warn!("Failed to create dir {:?}: {e}", full_path);
            }
        } else {
            if let Some(parent) = full_path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                tracing::warn!("Failed to create parent {:?}: {e}", parent);
            }
            if let Err(e) = std::fs::File::create(&full_path)
                .and_then(|mut f| std::io::copy(&mut entry, &mut f).map(|_| ()))
            {
                tracing::error!("Extract {:?}: {e}", out_path);
            }
        }
    }
}
