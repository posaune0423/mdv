use std::io::{self, IsTerminal, Write};

use anyhow::{Result, bail};

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

    let config = AppConfig::from(args);
    let source = FileSystemDocumentSource::new(config.path.clone());
    let content = source.read_to_string()?;
    let document = parse_document(config.path.clone(), &content)?;

    if io::stdout().is_terminal() && io::stdin().is_terminal() {
        if !crate::ui::terminal::is_supported_terminal(
            std::env::var("TERM_PROGRAM").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
        ) {
            bail!("interactive mode requires Ghostty or Kitty");
        }
        let mut viewer = TerminalViewer::new(config, source, document);
        viewer.run()?;
        return Ok(());
    }

    let rendered = render_plain_text(&document, config.theme, config.mermaid_mode);
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
