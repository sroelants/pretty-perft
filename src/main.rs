use clap::Parser;
use std::path::PathBuf;

mod components;
mod backends;
mod perft;
mod tui;

#[derive(Parser)]
#[command(author = "Sam Roelants", version = "0.1", about = "A simple perft tool.", long_about = None)]
struct Config {
    /// The desired search depth, in ply (half-turns)
    #[arg(short, long, default_value = "5")]
    depth: usize,

    #[arg(
        short,
        long,
        default_value = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    )]
    fen: String,

    #[arg(short, long)]
    engine: Option<PathBuf>,

    #[arg(short, long)]
    command: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    Config::parse().run()
}
