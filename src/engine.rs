use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use simbelmyne_chess::{board::Board, movegen::moves::Move};
use std::io::Write;

use super::perft::perft_divide;

type PerftResult = Vec<(Move, usize)>;

pub trait Perft {
    fn perft(
        &mut self,
        board: Board,
        depth: usize,
    ) -> anyhow::Result<PerftResult>;
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//
// Simbelmyne (built-in)
//
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

pub struct Simbelmyne {}

impl Perft for Simbelmyne {
    fn perft(
        &mut self,
        board: Board,
        depth: usize,
    ) -> anyhow::Result<PerftResult> {
        Ok(perft_divide(board, depth))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//
// Simbelmyne (external)
//
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//
// PerftThread
//
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct PerftRequest {
    board: Board,
    depth: usize,
    result_buf: Arc<Mutex<PerftResult>>,
}

pub struct PerftThread {
    tx: Sender<PerftRequest>
}

impl PerftThread {
    pub fn new<T: Perft + Send + 'static>(mut runner: T) -> Self {
        let (tx, rx) = channel::<PerftRequest>();

        std::thread::spawn(move || {
            for req in rx {
                let result = runner.perft(req.board, req.depth).unwrap();
                let mut buf = req.result_buf.lock().unwrap();
                *buf = result;
            }
        });

        Self { tx }
    }

    pub fn run(&mut self, board: Board, depth: usize, result_buf: Arc<Mutex<PerftResult>>) {
        self.tx.send(PerftRequest { board, depth, result_buf }).unwrap();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//
// Executable
//
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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
