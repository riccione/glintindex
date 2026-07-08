mod cli;
mod commands;

fn main() -> anyhow::Result<()> {
    cli::run()
}
