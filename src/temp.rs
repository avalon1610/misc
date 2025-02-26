use dirs::cache_dir;
use std::path::PathBuf;

pub fn temp_dir() -> PathBuf {
    let env_p = option_env!("BINARY_PKG_NAME");
    let p = env_p.unwrap_or(env!("CARGO_PKG_NAME"));
    cache_dir().unwrap_or_else(std::env::temp_dir).join(p)
}
