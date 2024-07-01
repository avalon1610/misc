use anyhow::Result;
use futures::{Future, StreamExt};
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook_tokio::{Handle, Signals};
use std::sync::Arc;
use tokio::{sync::Notify, task::JoinHandle};

pub fn setup_signal_handler<F, Fut>(notify: Arc<Notify>, proc: F) -> Result<Handler>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
{
    let signals = Signals::new([SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
    let handle = signals.handle();
    let task = tokio::spawn(handle_signals(signals, notify, proc));
    let handler = Handler { handle, task };
    Ok(handler)
}

pub struct Handler {
    handle: Handle,
    task: JoinHandle<()>,
}

impl Handler {
    pub async fn wait(self) -> Result<()> {
        self.handle.close();
        self.task.await?;
        Ok(())
    }
}

async fn handle_signals<F, Fut>(mut signals: Signals, notify: Arc<Notify>, proc: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future,
{
    if let Some(signal) = signals.next().await {
        log::info!("recv signal {}, exiting", signal);

        proc().await;
        notify.notify_waiters();
    }
}
