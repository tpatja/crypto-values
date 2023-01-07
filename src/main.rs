use crypto_values::get_updated_values;

use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Generator, Shell};

#[cfg(feature = "cli")]
use dotenv::dotenv;

use std::io;

#[derive(Parser, PartialEq, Debug)]
#[command(name = "crypto-values")]
#[command(author = "Teemu Patja <tp@iki.fi>")]
#[command(version = "0.1.0")]
#[command(about = "Gets crypto values from coinmarketcap API and uses a google sheet 
for managing holdings and tracking daily total value.", long_about = None)]
struct Cli {
    /// Generates shell completions for the given shell
    #[arg(long = "shell-completions", value_enum)]
    generator: Option<Shell>,

    /// Updates daily total value in google sheet if current value is higher than existing
    /// value or row for current date does not exist
    #[arg(short, long = "update-gsheet-total-value")]
    update: bool,

    /// Lists all holdings with current prices and total value
    #[arg(short, long = "show-prices")]
    show: bool,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "cli")]
    dotenv().ok();

    let opt = Cli::parse();
    if let Some(generator) = opt.generator {
        let mut cmd = Cli::command();
        print_completions(generator, &mut cmd);
    } else if opt.show {
        get_updated_values(true, false).await?;
    } else if opt.update {
        get_updated_values(false, true).await?;
    } else {
        Cli::command().print_help()?;
    }
    Ok(())
}
