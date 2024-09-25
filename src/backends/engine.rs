use std::{io::{self, BufRead, BufReader, Read, Write}, path::PathBuf, process::{Child, ChildStdin, ChildStdout, Command, Stdio}};
use anyhow::anyhow;

use simbelmyne_chess::{board::Board, movegen::moves::Move};

use super::{Perft, PerftResult};

pub struct Engine {
    child: Child,
    output: BufReader<ChildStdout>,
    input: ChildStdin,
}

impl Engine {
    pub fn new(path: PathBuf) -> io::Result<Engine> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut output = BufReader::new(child.stdout.take().expect("stdout not captured"));
        let mut input = child.stdin.take().expect("stdin not captured");

        writeln!(input, "uci")?;
        input.flush()?;


        for line in output.by_ref().lines() {
            if let Ok(line) = line {
                // println!("{line}");
                if &line == "uciok" {
                    break;
                }
            }
        }

        // // Initialize engine
        writeln!(input, "isready")?;
        input.flush()?;

        for line in output.by_ref().lines() {
            if let Ok(line) = line {
                if &line == "readyok" {
                    break;
                }
            }
        }

        Ok(Engine { child, input, output })
    }
}

impl Perft for Engine {
    fn perft(&mut self, board: Board, depth: usize) -> anyhow::Result<PerftResult> {
        // Set position
        writeln!(self.input, "position fen {}", board.to_fen())?;
        self.input.flush()?;

        writeln!(self.input, "isready")?;
        self.input.flush()?;

        for line in self.output.by_ref().lines() {
            if let Ok(line) = line {
                if &line == "readyok" {
                    break;
                }
            }
        }

        writeln!(self.input, "go perft {}\n", depth)?;
        self.input.flush()?;

        // parse child counts
        let mut move_list: PerftResult = Vec::new();

        for line in self.output.by_ref().lines() {
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


