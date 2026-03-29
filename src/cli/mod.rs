mod args;

pub use args::{MdvArgs, MdvCommand, Theme};

use clap::Parser;

#[must_use]
pub fn parse() -> MdvArgs {
    MdvArgs::parse()
}

#[must_use]
pub fn startup_message(args: &MdvArgs) -> String {
    if matches!(args.command.as_ref(), Some(MdvCommand::Update)) {
        return "mdv: command=update".to_string();
    }

    let path = args.path.as_deref().unwrap_or_else(|| std::path::Path::new("<missing>"));
    let mut message = format!("mdv: path={}", path.display());

    if args.watch {
        message.push_str(" watch=on");
    }

    if args.no_mermaid {
        message.push_str(" mermaid=off");
    }

    message.push_str(&format!(" theme={}", args.theme.as_str()));

    message
}
