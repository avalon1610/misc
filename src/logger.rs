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
    guard: TracingLoggerGuard,
}

pub struct TracingLoggerGuard {
    _non_blocking_guard: non_blocking::WorkerGuard,
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
        let app_name = pkg_name.replace('-', "_");
        let filter = format!("{}={},{}", app_name, verbose, default);
        let (layers, guard) = Self::default_layers(file_appender, filter);

        Ok(Self {
            layers,
            guard: TracingLoggerGuard {
                _non_blocking_guard: guard,
            },
        })
    }

    #[cfg(feature = "console")]
    pub fn enable_tokio_console(self) -> Self {
        self.add_layer(
            console_subscriber::Builder::default()
                .server_addr((std::net::Ipv4Addr::UNSPECIFIED, 6669))
                .spawn(),
        )
    }

    pub fn add_layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Registry> + Send + Sync,
    {
        self.layers = self.layers.and_then(layer).boxed();
        self
    }

    pub fn init(self) -> Result<TracingLoggerGuard> {
        registry().with(self.layers).init();
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
