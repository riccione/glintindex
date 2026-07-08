use clap::Args;

#[derive(Args)]
pub struct SearchArgs {
    /// The search query
    pub query: String,
}

pub fn execute(args: SearchArgs) -> anyhow::Result<()> {
    println!("Searching for: {}", args.query);
    Ok(())
}
