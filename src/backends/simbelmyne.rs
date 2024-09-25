use simbelmyne_chess::board::Board;

use crate::perft::perft_divide;

use super::{Perft, PerftResult};

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
