mod args;

pub use args::{MdvArgs, MdvCommand, Theme};

#[must_use]
pub fn parse() -> MdvArgs {
    let raw_args: Vec<_> = std::env::args_os().collect();

    if requested_version_flag(&raw_args) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    MdvArgs::parse_from(raw_args)
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

fn requested_version_flag(args: &[std::ffi::OsString]) -> bool {
    args.len() == 2 && args[1].to_str().is_some_and(|flag| matches!(flag, "--version" | "-V"))
}
