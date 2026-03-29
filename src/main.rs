fn main() {
    let args = mdv::cli::parse();

    if let Err(error) = mdv::app::run(args) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
