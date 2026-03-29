use std::sync::Once;

use tracing_subscriber::{EnvFilter, fmt};

static INIT: Once = Once::new();

pub fn init_tracing() {
    INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let _ = fmt().with_env_filter(filter).with_writer(std::io::stderr).try_init();
    });
}
