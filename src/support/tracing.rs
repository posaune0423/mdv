use std::sync::Once;

use tracing_subscriber::{EnvFilter, fmt, fmt::format::FmtSpan};

static INIT: Once = Once::new();

pub fn init_tracing() {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let spans = if std::env::var_os("MDV_PROFILE_STARTUP").is_some() {
            FmtSpan::CLOSE
        } else {
            FmtSpan::NONE
        };
        let _ = fmt()
            .with_env_filter(filter)
            .with_span_events(spans)
            .with_writer(std::io::stderr)
            .try_init();
    });
}
