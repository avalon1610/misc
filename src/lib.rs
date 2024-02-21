#[cfg(any(feature = "async", feature = "tracing_logger"))]
use anyhow::Result;
#[cfg(feature = "async")]
use anyhow::{anyhow, Context};
#[cfg(feature = "async")]
use log::warn;
#[cfg(feature = "random")]
use rand::{distributions::Alphanumeric, Rng};
#[cfg(feature = "async")]
use serde::{de::DeserializeOwned, Serialize};
use std::borrow::Cow;
#[cfg(feature = "async")]
use std::{
    future::Future,
    path::Path,
    sync::{mpsc, Arc, Condvar, Mutex},
};
#[cfg(feature = "async")]
use tokio::{fs, runtime::Runtime};
#[cfg(feature = "nom_err")]
pub mod nom;
#[cfg(feature = "panic_handler")]
pub mod panic;
#[cfg(feature = "signals")]
pub mod signals;

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
                error!("{:?}", e);
            }
        }
    };
}

#[cfg(all(feature = "config_json", feature = "config_toml"))]
compile_error!(
    "feature \"config_json\" and feature \"config_toml\" cannot be enabled at the same time"
);

#[cfg(any(feature = "config_json", feature = "config_toml"))]
#[async_trait::async_trait]
pub trait ConfigManager
where
    Self: Default + DeserializeOwned + Serialize,
{
    async fn load(file: impl AsRef<Path> + Send + Sync + 'async_trait) -> Result<Self> {
        let cfg = fs::read(file.as_ref())
            .await
            .context("ConfigManager::load read file failed")?;

        deserialize(cfg)
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
        let cfg = serialize(self)?;
        fs::write(file.as_ref(), cfg.as_bytes()).await?;

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

#[cfg(feature = "async")]
impl<T> ConfigManager for T where T: Serialize + DeserializeOwned + Default + Send {}

#[cfg(feature = "logger")]
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

#[cfg(feature = "tracing_logger")]
pub fn init_tracing_logger(
    log_dir: impl AsRef<std::path::Path>,
    pkg_name: &str,
    debug: bool,
    default: &str,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_appender::rolling::{RollingFileAppender, Rotation};
    use tracing_subscriber::{fmt::time::OffsetTime, prelude::*};

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(pkg_name)
        .filename_suffix("log")
        .build(log_dir)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let log_level = if debug {
        format!("{}=debug,{}", pkg_name.replace('-', "_"), default)
    } else {
        default.to_owned()
    };

    let timer = OffsetTime::new(
        time::macros::offset!(+8),
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]"),
    );
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer().with_timer(timer.clone()))
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(timer)
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    Ok(guard)
}

pub trait ToUtf8String {
    fn to_utf8_lossy(&self) -> Cow<'_, str>;
}

impl ToUtf8String for [u8] {
    fn to_utf8_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self)
    }
}

#[cfg(feature = "random")]
pub fn rand_string(count: usize) -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(count)
        .map(char::from)
        .collect()
}

#[cfg(feature = "async")]
#[allow(clippy::mutex_atomic)]
pub fn block_spawn<F, T>(f: F) -> Result<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let t = std::thread::spawn(move || {
        let cv1 = Arc::new((Mutex::new(false), Condvar::new()));
        let cv2 = cv1.clone();
        let runtime = Runtime::new().unwrap();
        let (tx, rx) = mpsc::channel();
        runtime.spawn(async move {
            let output = f.await;
            tx.send(output).unwrap();

            let (lock, cv) = &*cv2;
            let mut end = lock.lock().unwrap();
            *end = true;
            cv.notify_one();
        });

        let (lock, cv) = &*cv1;
        let mut end = lock.lock().unwrap();
        while !*end {
            end = cv.wait(end).unwrap();
        }

        rx.recv().unwrap()
    });

    t.join().map_err(|e| anyhow!("{:?}", e))
}

#[cfg(test)]
mod test {
    use crate::{block_spawn, ToUtf8String};
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_block_spawn() {
        let r = block_spawn(async {
            for i in 0..10 {
                println!("task run {}", i);
                sleep(Duration::from_millis(500)).await;
            }

            return "hello world".to_string();
        })
        .unwrap();

        println!("task done: {}", r);
    }

    #[test]
    fn test_to_utf8_lossy() {
        let a = [0x31u8, 0x32u8, 0x33u8];
        let b = a[..2].to_utf8_lossy().to_string();
        assert_eq!("12", b);

        let c = vec![0x31u8, 0x32u8, 0x33u8];
        let d = c.to_utf8_lossy().to_string();
        assert_eq!("123", d);
    }
}
