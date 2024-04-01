use std::path::PathBuf;
use clap::Parser;
use tui::init_tui;

mod components;
mod board_view;
mod diff_table;
mod engine;
mod info_view;
mod tui;
mod perft;


#[derive(Parser)]
#[command(author = "Sam Roelants", version = "0.1", about = "A simple perft tool.", long_about = None)]
struct Cli {
    /// The desired search depth, in ply (half-turns)
    #[arg(short, long, default_value = "5")]
    depth: usize,

    #[arg(short, long, default_value = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")]
    fen: String,

    #[arg(short, long)]
    engine: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { depth, fen, engine } = Cli::parse();

    init_tui(depth, fen, engine).await
}
