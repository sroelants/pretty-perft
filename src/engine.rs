use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::path::PathBuf;
use std::io::{self, BufRead, BufReader, Read};

use simbelmyne_chess::{board::Board, movegen::moves::Move};
use std::io::Write;
use anyhow::anyhow;

use super::perft::perft_divide;

type PerftResult = Vec<(Move, usize)>;

pub trait Perft {
    async fn perft(&mut self, board: Board, depth: usize) -> anyhow::Result<PerftResult>;
}

pub struct Simbelmyne {}

impl Perft for Simbelmyne {
    async fn perft(&mut self, board: Board, depth: usize) -> anyhow::Result<PerftResult> {
        let task = tokio::task::spawn_blocking(move || {
            let move_list: PerftResult = perft_divide(board, depth)
                .into_iter()
                .map(|(mv, nodes)| (mv, nodes))
                .collect();

            Ok(move_list)
        });

        task.await?
    }
}

pub struct Engine {
    child: Child,
    inp: BufReader<ChildStdout>,
    out: ChildStdin,
}

impl Engine {
    pub fn new(path: PathBuf) -> io::Result<Engine> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut inp = BufReader::new(child.stdout.take().expect("stdout not captured"));

        let mut out = child.stdin.take().expect("stdin not captured");

        // Initialize engine
        write!(out, "uci")?;

        for line in inp.by_ref().lines() {
            if let Ok(line) = line {
                if &line == "uciok" {
                    break;
                }
            }
        }

        Ok(Engine { child, inp, out })
    }
}

impl Perft for Engine {
    async fn perft(&mut self, board: Board, depth: usize) -> anyhow::Result<PerftResult> {

        // Initialize engine
        write!(self.out, "isready")?;

        for line in self.inp.by_ref().lines() {
            if let Ok(line) = line {
                if &line == "readyok" {
                    break;
                }
            }
        }

        // Set position
        write!(self.out, "position fen {}", board.to_fen())?;

        write!(self.out, "\ngo perft {}\n", depth)?;

        // parse child counts
        let mut move_list: PerftResult = Vec::new();

        for line in self.inp.by_ref().lines() {
            let line = line?;

            if line.trim().is_empty() {
                break;
            } else {
                let mut parts = line.trim().split(": ");

                let move_ = parts.next()
                    .ok_or(anyhow!("Failed to parse perft output {line}"))?
                    .to_string();

                let count = parts
                    .next()
                    .ok_or(anyhow!("Failed to parse perft output {line}"))?
                    .parse().map_err(|_| anyhow!("failed to parse perft output {line}"))?;

                let mv: Move = move_.parse()?;
                move_list.push((mv, count));
            }
        }

        Ok(move_list)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
