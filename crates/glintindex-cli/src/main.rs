mod cli;
mod commands;
mod progress;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
