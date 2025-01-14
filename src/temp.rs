use dirs::cache_dir;
use std::{
    env::var,
    path::{Path, PathBuf},
};

pub fn temp_dir() -> PathBuf {
    let env_p = var("BINARY_PKG_NAME");
    let env_p = env_p.as_ref().map(|s| s.as_str());

    let p = Path::new(env_p.unwrap_or(env!("CARGO_PKG_NAME")));
    cache_dir().unwrap_or_else(std::env::temp_dir).join(p)
}
