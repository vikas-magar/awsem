pub fn validate(name: &str) -> Result<(), &'static str> {
    if name.is_empty() { return Err("name must not be empty"); }
    if name.len() > 64 { return Err("name must be at most 64 characters"); }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("name can only contain letters, numbers, hyphens, and underscores");
    }
    Ok(())
}

pub fn data_dir_fn(data_dir: &Option<String>, name: &str) -> String {
    let base = data_dir.clone().unwrap_or_else(|| "./lambdas".into());
    format!("{base}/{name}")
}
