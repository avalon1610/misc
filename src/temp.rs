use dirs::cache_dir;
use log::error;
use std::{fs::create_dir_all, path::PathBuf};

pub fn temp_dir() -> PathBuf {
    let env_p = option_env!("BINARY_PKG_NAME");
    let p = env_p.unwrap_or(env!("CARGO_PKG_NAME"));
    let path = cache_dir().unwrap_or_else(std::env::temp_dir).join(p);

    if !path.exists() {
        if let Err(e) = create_dir_all(&path) {
            error!("create temp dir {} error: {:?}", path.display(), e);
        }
    }

    path
}
