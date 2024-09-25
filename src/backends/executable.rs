use std::{path::PathBuf, process::Command};
use anyhow::anyhow;

use simbelmyne_chess::{board::Board, movegen::moves::Move};

use super::{Perft, PerftResult};

pub struct Executable {
    path: PathBuf
}

impl Executable {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Perft for Executable {
    fn perft(&mut self, board: Board, depth: usize) -> anyhow::Result<PerftResult> {
        let output_bytes = Command::new(&self.path)
            .arg(board.to_fen())
            .arg(depth.to_string())
            .output()
            .unwrap();

        let output = String::from_utf8(output_bytes.stdout)?;
        let mut move_list: PerftResult = Vec::new();

        for line in output.lines() {
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

