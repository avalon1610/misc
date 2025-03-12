#![allow(async_fn_in_trait)]
use anyhow::{Context, Result};
use log::warn;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

#[cfg(any(
    feature = "config_json",
    feature = "config_toml",
    feature = "config_bin"
))]
pub trait ConfigType<T: Serialize + DeserializeOwned> {
    fn serialize(v: &T) -> Result<Vec<u8>>;
    fn deserialize(cfg: Vec<u8>) -> Result<T>;
}

#[cfg(any(
    feature = "config_json",
    feature = "config_toml",
    feature = "config_bin"
))]
pub trait ConfigManager
where
    Self: Default + DeserializeOwned + Serialize,
{
    type ImplType: ConfigType<Self>;

    #[cfg(feature = "async")]
    async fn load(file: impl AsRef<Path> + Send + Sync) -> Result<Self> {
        let cfg = tokio::fs::read(file.as_ref())
            .await
            .context("ConfigManager::load read file failed")?;

        Self::ImplType::deserialize(cfg)
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
        let cfg = Self::ImplType::serialize(self)?;
        tokio::fs::write(file.as_ref(), cfg).await?;

        Ok(())
    }

    fn load_sync(file: impl AsRef<Path> + Send + Sync) -> Result<Self> {
        let cfg = std::fs::read(file.as_ref()).context("ConfigManager::load read file failed")?;

        Self::ImplType::deserialize(cfg)
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
        let cfg = Self::ImplType::serialize(self)?;
        std::fs::write(file.as_ref(), cfg)?;

        Ok(())
    }
}

#[cfg(feature = "config_toml")]
pub struct ConfigToml;

#[cfg(feature = "config_toml")]
impl<T> ConfigType<T> for ConfigToml
where
    T: Serialize + DeserializeOwned,
{
    fn deserialize(cfg: Vec<u8>) -> Result<T> {
        toml::from_str(&String::from_utf8_lossy(&cfg))
            .context("ConfigManager::load deserialize failed")
    }

    fn serialize(v: &T) -> Result<Vec<u8>> {
        Ok(toml::to_string_pretty(v).map(String::into_bytes)?)
    }
}

#[cfg(feature = "config_json")]
pub struct ConfigJson;

#[cfg(feature = "config_json")]
impl<T> ConfigType<T> for ConfigJson
where
    T: Serialize + DeserializeOwned,
{
    fn deserialize(cfg: Vec<u8>) -> Result<T> {
        serde_json::from_slice(&cfg).context("ConfigManager::load deserialize failed")
    }

    fn serialize(v: &T) -> Result<Vec<u8>> {
        Ok(serde_json::to_string_pretty(v).map(String::into_bytes)?)
    }
}

#[cfg(feature = "config_bin")]
pub struct ConfigBin;

#[cfg(feature = "config_bin")]
impl<T> ConfigType<T> for ConfigBin
where
    T: Serialize + DeserializeOwned,
{
    fn serialize(v: &T) -> Result<Vec<u8>> {
        Ok(bincode::serde::encode_to_vec(
            v,
            bincode::config::standard(),
        )?)
    }

    fn deserialize(cfg: Vec<u8>) -> Result<T> {
        let (r, _) = bincode::serde::decode_from_slice(&cfg, bincode::config::standard())?;
        Ok(r)
    }
}
