use anyhow::{Context, Result};
use log::warn;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tokio::fs;

#[macro_export]
macro_rules! async_block {
    ($block: block) => {
        async move {
            if let Err(e) = async move {
                $block;

                #[allow(unreachable_code)]
                Ok::<_, anyhow::Error>(())
            }
            .await
            {
                log::error!("{:?}", e);
            }
        }
    };
}

#[async_trait::async_trait]
pub trait ConfigManager
where
    Self: Default + DeserializeOwned + Serialize,
{
    async fn load(file: impl AsRef<Path> + Send + Sync + 'async_trait) -> Result<Self> {
        let cfg = fs::read(file.as_ref())
            .await
            .context("ConfigManager::load read file failed")?;
        Ok(serde_json::from_slice(&*cfg).context("ConfigManager::load deserialize failed")?)
    }

    async fn load_or_default(file: impl AsRef<Path> + Send + Sync + 'async_trait) -> Self {
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

    async fn save(
        &self,
        file: impl AsRef<Path> + Send + Sync + 'async_trait,
    ) -> anyhow::Result<()> {
        let cfg = serde_json::to_string_pretty(self)?;
        fs::write(file.as_ref(), cfg.as_bytes()).await?;

        Ok(())
    }
}

impl<T> ConfigManager for T where T: Serialize + DeserializeOwned + Default + Send {}

pub fn init_env_logger(pkg_name: &str, debug: bool, default: &str) {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            format!(
                "{}={},{}",
                pkg_name.replace('-', "_"),
                if debug { "debug" } else { default },
                default
            ),
        );
    }
    env_logger::init();
}

pub trait ToUtf8String {
    fn to_utf8_lossy(self) -> String;
}

impl ToUtf8String for &[u8] {
    fn to_utf8_lossy(self) -> String {
        String::from_utf8_lossy(self).to_string()
    }
}