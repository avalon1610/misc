use anyhow::Result;
use time::macros::{format_description, offset};
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::OffsetTime},
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

#[cfg(feature = "tracing_logger")]
pub struct TracingLogger {
    layers: Box<dyn Layer<Registry> + Send + Sync>,
    #[allow(dead_code)]
    console: bool,
    guard: non_blocking::WorkerGuard,
}

#[cfg(feature = "tracing_logger")]
impl TracingLogger {
    pub fn new(
        log_dir: impl AsRef<std::path::Path>,
        pkg_name: &str,
        verbose: &str,
        default: &str,
    ) -> Result<Self> {
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(pkg_name)
            .filename_suffix("log")
            .build(log_dir)?;
        let filter = format!("{}={},{}", pkg_name.replace('-', "_"), verbose, default);
        let (layers, guard) = Self::default_layers(file_appender, filter);

        Ok(Self {
            layers,
            guard,
            console: false,
        })
    }

    #[cfg(feature = "console")]
    pub fn enable_tokio_console(mut self) -> Self {
        self.console = true;
        self
    }

    pub fn add_layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Registry> + Send + Sync,
    {
        self.layers = self.layers.and_then(layer).boxed();
        self
    }

    pub fn init(self) -> Result<non_blocking::WorkerGuard> {
        let layered = registry().with(self.layers);

        #[cfg(feature = "console")]
        if self.console {
            layered
                .with(
                    console_subscriber::Builder::default()
                        .server_addr((std::net::Ipv4Addr::UNSPECIFIED, 6669))
                        .spawn(),
                )
                .init();
            return Ok(self.guard);
        }

        layered.init();
        Ok(self.guard)
    }

    fn default_layers(
        file_appender: RollingFileAppender,
        filter: String,
    ) -> (
        Box<dyn Layer<Registry> + Send + Sync>,
        non_blocking::WorkerGuard,
    ) {
        let (non_blocking, guard) = non_blocking(file_appender);
        let timer = OffsetTime::new(
            offset!(+8),
            format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]"),
        );
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into());
        
        let file_layer = fmt::layer()
            .compact()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_timer(timer.clone())
            .with_span_events(FmtSpan::NEW);
        let stdout_layer = fmt::layer()
            .compact()
            .with_timer(timer)
            .with_span_events(FmtSpan::NEW);

        (
            file_layer
                .and_then(stdout_layer)
                .with_filter(filter)
                .boxed(),
            guard,
        )
    }
}
