#[derive(Debug, Clone)]
pub struct RustFsConfig {
    pub namespace: String,
    pub image: String,
    pub pvc_size: String,
}

impl RustFsConfig {
    pub fn new(namespace: &str, image: &str, pvc_size: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            image: image.to_string(),
            pvc_size: pvc_size.to_string(),
        }
    }
}
