use dirs::cache_dir;
use std::path::PathBuf;

pub fn temp_dir() -> PathBuf {
    let env_p = env!("BINARY_PKG_NAME");
    let p = if env_p.is_empty() {
        env!("CARGO_PKG_NAME")
    } else {
        env_p
    };

    cache_dir().unwrap_or_else(std::env::temp_dir).join(p)
}
