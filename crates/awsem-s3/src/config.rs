#[derive(Debug, Clone)]
pub struct RustFsConfig {
    pub namespace: String,
    pub image: String,
    pub pvc_size: String,
    pub access_key: String,
    pub secret_key: String,
}

impl RustFsConfig {
    pub fn new(namespace: &str, image: &str, pvc_size: &str, access_key: &str, secret_key: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            image: image.to_string(),
            pvc_size: pvc_size.to_string(),
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
        }
    }
}
