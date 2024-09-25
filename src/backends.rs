use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use simbelmyne_chess::{board::Board, movegen::moves::Move};

mod simbelmyne;
mod engine;
mod executable;

pub use simbelmyne::*;
pub use engine::*;
pub use executable::*;

type PerftResult = Vec<(Move, usize)>;

pub trait Perft {
    fn perft(
        &mut self,
        board: Board,
        depth: usize,
    ) -> anyhow::Result<PerftResult>;
}

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
