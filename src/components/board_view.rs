use simbelmyne_chess::constants::LIGHT_SQUARES;
use simbelmyne_chess::square::Square;
use simbelmyne_chess::{board::Board, movegen::moves::Move};
use simbelmyne_chess::piece::Piece;
use itertools::Itertools;
use ratatui::{
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};

pub struct BoardView {
    pub board: Board,
    pub highlight: Option<Move>,
}

fn square_to_cell(piece: Option<Piece>) -> Cell<'static> {
    match piece {
        Some(piece) => to_padded_cell(piece.to_string()),
        None => to_padded_cell(String::from("")),
    }
}

const CELL_WIDTH: usize = 5;
const CELL_HEIGHT: usize = 3;

fn to_padded_cell(val: String) -> Cell<'static> {
    let lines = vec![
        vec![Line::from(""); CELL_HEIGHT / 2],
        vec![Line::from(format!("{:^CELL_WIDTH$}", val))],
        vec![Line::from(""); CELL_HEIGHT / 2],
    ]
    .concat();

    Cell::from(lines)
}

impl Widget for BoardView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 10 * CELL_WIDTH;
        let height = 10 * CELL_HEIGHT;

        let rect = Layout::new(
            Direction::Vertical, 
            [
                Constraint::Min((area.height - height as u16) / 2),
                Constraint::Min(height as u16),
                Constraint::Min((area.height - height as u16) / 2),
            ])
            .split(area)[1];

        let rect = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Min((area.width - width as u16) / 2),
                Constraint::Min(width as u16),
                Constraint::Min((area.width - width as u16) / 2),
            ])
            .split(rect)[1];

        let file_labels = vec!["", "a", "b", "c", "d", "e", "f", "g", "h", ""]
            .into_iter()
            .map(|label| to_padded_cell(label.to_owned()))
            .collect_vec();

        let file_labels = Row::new(file_labels).height(CELL_HEIGHT as u16).dark_gray();

        let mut rows: Vec<Row> = Vec::new();
        // Push top heading
        rows.push(file_labels.clone());

        let mut current_rank: Vec<Cell> = Vec::new();
        let ranks = self.board.piece_list.into_iter().chunks(8);
        let ranks = ranks
            .into_iter()
            .enumerate()
            .collect_vec()
            .into_iter()
            .rev();

        for (rank, squares) in ranks {
            let rank_label = to_padded_cell((rank + 1).to_string()).dark_gray();
            current_rank.push(rank_label.clone());

            for (file, piece) in squares.enumerate() {
                let sq = Square::from(8 * rank + file);

                let cell = if self.highlight.is_some_and(|mv| sq == mv.src()) {
                    square_to_cell(piece).on_blue()
                } else if self.highlight.is_some_and(|mv| sq == mv.tgt()) {
                    square_to_cell(piece).on_blue()
                } else if LIGHT_SQUARES.contains(sq) {
                    square_to_cell(piece)
                } else {
                    square_to_cell(piece).on_dark_gray()
                };

                current_rank.push(cell);
            }

            current_rank.push(rank_label);

            rows.push(Row::new(current_rank).height(CELL_HEIGHT as u16));
            current_rank = Vec::new();
        }

        // Push bottom heading
        rows.push(file_labels);

        let table = Table::new(
            rows,
            &[Constraint::Length(CELL_WIDTH as u16); 10]
        )
            .column_spacing(0);

        let border = Block::new()
            .title("Board")
            .borders(Borders::ALL)
            .title_style(Style::new().white())
            .border_style(Style::new().dark_gray());

        Widget::render(border, area, buf);
        Widget::render(table, rect, buf);
    }
}
