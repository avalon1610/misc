use anyhow::{anyhow, Context, Result};
pub use http::{HttpContext, HttpError, HttpResult};
use log::warn;
use rand::{distributions::Alphanumeric, Rng};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    future::Future,
    path::Path,
    sync::{mpsc, Arc, Condvar, Mutex},
};
use tokio::{fs, runtime::Runtime};

mod http;

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

pub fn rand_string(count: usize) -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(count)
        .map(char::from)
        .collect()
}

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
    use crate::block_spawn;
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
}
