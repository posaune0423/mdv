use std::{
    io::{self, IsTerminal, Read, Write},
    path::Path,
};

use anyhow::{Result, bail};
use tracing::info_span;

use crate::{
    cli::{MdvArgs, MdvCommand},
    core::config::AppConfig,
    io::{fs::FileSystemDocumentSource, self_update},
    render::{markdown::parse_document, text::render_plain_text},
    support::tracing::init_tracing,
    ui::terminal::TerminalViewer,
};

pub fn run(args: MdvArgs) -> Result<()> {
    init_tracing();

    let _startup_span = info_span!("app.run").entered();
    if matches!(args.command.as_ref(), Some(MdvCommand::Update)) {
        return self_update::update_current_executable();
    }

    let config = AppConfig::try_from(args)?;
    if config.path == Path::new("-") {
        return run_from_stdin(config);
    }

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
            bail!("interactive mode requires Ghostty, bcon, or Kitty");
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

fn run_from_stdin(config: AppConfig) -> Result<()> {
    if io::stdin().is_terminal() {
        bail!("'-' requires piped stdin");
    }

    let mut content = String::new();
    {
        let _span = info_span!("startup.read_stdin").entered();
        io::stdin().read_to_string(&mut content)?;
    }

    let document = {
        let _span = info_span!("startup.parse_document").entered();
        let virtual_path = std::env::current_dir()?.join("stdin.md");
        parse_document(virtual_path, &content)?
    };

    let rendered = {
        let _span = info_span!("startup.render_plain_text").entered();
        render_plain_text(&document, config.theme, config.mermaid_mode)
    };
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
