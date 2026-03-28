mod args;

pub use args::{MdvArgs, Theme};

use clap::Parser;

#[must_use]
pub fn parse() -> MdvArgs {
    MdvArgs::parse()
}

#[must_use]
pub fn startup_message(args: &MdvArgs) -> String {
    let mut message = format!("mdv: path={}", args.path.display());

    if args.watch {
        message.push_str(" watch=on");
    }

    if args.no_mermaid {
        message.push_str(" mermaid=off");
    }

    message.push_str(&format!(" theme={}", args.theme.as_str()));

    message
}
