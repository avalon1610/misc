#[cfg(feature = "async")]
use anyhow::anyhow;
use std::borrow::Cow;
#[cfg(feature = "async")]
use std::{
    future::Future,
    sync::{mpsc, Arc, Condvar, Mutex},
};
#[cfg(feature = "async")]
use tokio::runtime::Runtime;
#[cfg(any(
    feature = "config_json",
    feature = "config_toml",
    feature = "config_bin"
))]
pub mod config;
#[cfg(feature = "tracing_logger")]
mod logger;
#[cfg(feature = "tracing_logger")]
pub use logger::TracingLogger;
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

#[cfg(feature = "logger")]
pub fn init_env_logger(pkg_name: &str, verbose: &str, default: &str) {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            format!("{}={},{}", pkg_name.replace('-', "_"), verbose, default),
        );
    }
    env_logger::init();
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
    rand::Rng::sample_iter(rand::thread_rng(), rand::distributions::Alphanumeric)
        .take(count)
        .map(char::from)
        .collect()
}

#[cfg(feature = "async")]
#[allow(clippy::mutex_atomic)]
pub fn block_spawn<F, T>(f: F) -> anyhow::Result<T>
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

#[cfg(feature = "async")]
pub async fn loop_task<P, F>(
    name: &'static str,
    proc: P,
    interval: u64,
    notify: Arc<tokio::sync::Notify>,
) -> impl Future
where
    P: Fn() -> F,
    F: Future,
{
    async move {
        let task = async move {
            loop {
                if interval > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                }
                proc().await;
            }
        };

        tokio::select! {
            _ = task => {}
            _ = notify.notified() => {
                log::debug!("loop task {} notified and exited", name);
            }
        }
    }
}

#[cfg(feature = "sink")]
pub trait IntoAsyncWrite
where
    Self: futures::Sink<bytes::Bytes> + Sized,
{
    fn into_async_write(self) -> impl tokio::io::AsyncWrite {
        use futures::SinkExt;

        tokio_util::io::SinkWriter::new(tokio_util::io::CopyToBytes::new(
            self.sink_map_err(|_| std::io::Error::from(std::io::ErrorKind::BrokenPipe)),
        ))
    }
}

#[cfg(feature = "sink")]
impl<T> IntoAsyncWrite for T where T: futures::Sink<bytes::Bytes> + Sized {}

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
