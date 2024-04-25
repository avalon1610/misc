#![allow(async_fn_in_trait)]

use anyhow::Context;
use anyhow::Result;
use log::warn;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

#[cfg(all(feature = "config_json", feature = "config_toml"))]
compile_error!(
    "feature \"config_json\" and feature \"config_toml\" cannot be enabled at the same time"
);

#[cfg(any(feature = "config_json", feature = "config_toml"))]
pub trait ConfigManager
where
    Self: Default + DeserializeOwned + Serialize,
{
    #[cfg(feature = "async")]
    async fn load(file: impl AsRef<Path> + Send + Sync) -> Result<Self> {
        let cfg = tokio::fs::read(file.as_ref())
            .await
            .context("ConfigManager::load read file failed")?;

        deserialize(cfg)
    }

    #[cfg(feature = "async")]
    async fn load_or_default(file: impl AsRef<Path> + Send + Sync) -> Self {
        match Self::load(file).await {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "load failed: {:?}.\nusing default of [{}]",
                    e,
                    std::any::type_name::<Self>(),
                );
                Self::default()
            }
        }
    }

    #[cfg(feature = "async")]
    async fn save(&self, file: impl AsRef<Path> + Send + Sync) -> anyhow::Result<()> {
        let cfg = serialize(self)?;
        tokio::fs::write(file.as_ref(), cfg.as_bytes()).await?;

        Ok(())
    }

    fn load_sync(file: impl AsRef<Path> + Send + Sync) -> Result<Self> {
        let cfg = std::fs::read(file.as_ref()).context("ConfigManager::load read file failed")?;

        deserialize(cfg)
    }

    fn load_or_default_sync(file: impl AsRef<Path> + Send + Sync) -> Self {
        match Self::load_sync(file) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "load failed: {:?}.\nusing default of [{}]",
                    e,
                    std::any::type_name::<Self>(),
                );
                Self::default()
            }
        }
    }

    fn save_sync(&self, file: impl AsRef<Path> + Send + Sync) -> anyhow::Result<()> {
        let cfg = serialize(self)?;
        std::fs::write(file.as_ref(), cfg.as_bytes())?;

        Ok(())
    }
}

#[cfg(feature = "config_json")]
fn deserialize<T: DeserializeOwned>(cfg: Vec<u8>) -> Result<T> {
    serde_json::from_slice(&cfg).context("ConfigManager::load deserialize failed")
}

#[cfg(feature = "config_json")]
fn serialize<T: Serialize>(v: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(v)?)
}

#[cfg(feature = "config_toml")]
fn deserialize<T: DeserializeOwned>(cfg: Vec<u8>) -> Result<T> {
    toml::from_str(&String::from_utf8_lossy(&cfg)).context("ConfigManager::load deserialize failed")
}

#[cfg(feature = "config_toml")]
fn serialize<T: Serialize>(v: &T) -> Result<String> {
    Ok(toml::to_string_pretty(v)?)
}

#[cfg(any(feature = "config_json", feature = "config_toml"))]
impl<T> ConfigManager for T where T: Serialize + DeserializeOwned + Default + Send {}
