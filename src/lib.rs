use std::borrow::Cow;
#[cfg(feature = "async")]
use std::{future::Future, sync::Arc};
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
pub fn spawn_look_task<P, F>(
    name: &'static str,
    proc: P,
    interval: u64,
    notify: Arc<tokio::sync::Notify>,
) where
    P: Fn() -> F + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let task = async move {
            loop {
                proc().await;
                if interval > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                }

                if interval == 0 {
                    break;
                }
            }
        };

        tokio::select! {
            _ = task => {}
            _ = notify.notified() => {
                log::debug!("loop task {} notified and exited", name);
            }
        }
    });
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
    use crate::ToUtf8String;
    use tokio::time::{sleep, Duration};

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_spawn_loop_task() {
        std::env::set_var("RUST_LOG", "info");
        env_logger::init();

        let notify = std::sync::Arc::new(tokio::sync::Notify::new());
        crate::spawn_look_task(
            "test",
            || async {
                log::info!("loop task run");
            },
            1,
            notify.clone(),
        );

        sleep(Duration::from_secs(5)).await;
        notify.notify_one();
        sleep(Duration::from_secs(1)).await;
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
