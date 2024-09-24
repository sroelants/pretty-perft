use clap::Parser;
use std::path::PathBuf;
use tui::init_tui;

mod board_view;
mod components;
mod diff_table;
mod engine;
mod info_view;
mod perft;
mod tui;

#[derive(Parser)]
#[command(author = "Sam Roelants", version = "0.1", about = "A simple perft tool.", long_about = None)]
struct Cli {
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
    engine: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let Cli { depth, fen, engine } = Cli::parse();
    init_tui(depth, fen, engine)
}
