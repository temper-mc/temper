pub mod errors;

use std::path::Path;
use temper_general_purpose::paths::get_root_path;
use temper_profiling::ProfilerTracingLayer;
use tracing::{Level, Subscriber};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub fn init_logging(trace_level: Level) {
    //let console = console_subscriber::spawn();
    let env_filter = EnvFilter::builder()
        .with_default_directive(trace_level.into())
        .parse_lossy("");

    // Disallow request and hyper-util debug prints since they spam the console
    let env_filter = env_filter
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("h2=warn".parse().unwrap())
        .add_directive("rustls=warn".parse().unwrap())
        .add_directive("tungstenite=warn".parse().unwrap())
        .add_directive("hyper_util=warn".parse().unwrap());

    let file_layer = file_layer(get_root_path().join("logs"));

    let profiler_layer = ProfilerTracingLayer;

    let registry = tracing_subscriber::registry()
        .with(file_layer)
        .with(env_filter)
        .with(profiler_layer);

    #[cfg(feature = "tracy")]
    let tracy_layer = tracing_tracy::TracyLayer::default();
    // Registry becomes a different type when a layer is added, so we need to
    // shadow it here.
    #[cfg(feature = "tracy")]
    let registry = registry.with(tracy_layer);

    let layer = {
        #[cfg(debug_assertions)]
        {
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_level(true)
                .with_target(false)
        }
        #[cfg(not(debug_assertions))]
        {
            tracing_subscriber::fmt::layer()
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
                .with_level(true)
                .with_target(false)
        }
    };
    registry.with(layer).init();
}

/// Builds the daily rolling file layer that writes into `log_dir`.
///
/// Returns `None` if the log directory can't be created (for example when the
/// binary sits in a read-only directory), so the server still starts with
/// console logging only.
fn file_layer<S>(log_dir: impl AsRef<Path>) -> Option<impl Layer<S>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let file_appender = match tracing_appender::rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("temper")
        .filename_suffix("log.txt")
        .build(log_dir.as_ref())
    {
        Ok(file_appender) => file_appender,
        Err(e) => {
            eprintln!(
                "Could not create log directory {}, logging to console only: {e}",
                log_dir.as_ref().display()
            );
            return None;
        }
    };

    Some(
        tracing_subscriber::fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::Registry;

    #[test]
    fn file_layer_is_none_when_log_dir_cannot_be_created() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let not_a_dir = temp_dir.path().join("not_a_dir");
        std::fs::write(&not_a_dir, b"").expect("file should be created");

        assert!(file_layer::<Registry>(not_a_dir.join("logs")).is_none());
    }
}
