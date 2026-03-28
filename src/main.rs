fn main() {
    let args = mdv::cli::parse();
    println!("{}", mdv::cli::startup_message(&args));
}
