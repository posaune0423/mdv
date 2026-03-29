use std::io::{self, IsTerminal, Write};

use anyhow::{Result, bail};
use tracing::info_span;

use crate::{
    cli::MdvArgs,
    core::config::AppConfig,
    io::fs::FileSystemDocumentSource,
    render::{markdown::parse_document, text::render_plain_text},
    support::tracing::init_tracing,
    ui::terminal::TerminalViewer,
};

pub fn run(args: MdvArgs) -> Result<()> {
    init_tracing();

    let _startup_span = info_span!("app.run").entered();
    let config = AppConfig::from(args);
    let source = FileSystemDocumentSource::new(config.path.clone());
    let content = {
        let _span = info_span!("startup.read_source").entered();
        source.read_to_string()?
    };
    let document = {
        let _span = info_span!("startup.parse_document").entered();
        parse_document(config.path.clone(), &content)?
    };

    if io::stdout().is_terminal() && io::stdin().is_terminal() {
        if !crate::ui::terminal::is_supported_terminal(
            std::env::var("TERM_PROGRAM").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
        ) {
            bail!("interactive mode requires Ghostty or Kitty");
        }
        let mut viewer = {
            let _span = info_span!("startup.create_viewer").entered();
            TerminalViewer::try_new(config, source, document, content)?
        };
        viewer.run()?;
        return Ok(());
    }

    let rendered = {
        let _span = info_span!("startup.render_plain_text").entered();
        render_plain_text(&document, config.theme, config.mermaid_mode)
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
