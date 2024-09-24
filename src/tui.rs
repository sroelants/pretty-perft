pub(crate) use std::collections::BTreeMap;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::KeyCode;
use ratatui::prelude::Constraint;
use ratatui::{
    prelude::{CrosstermBackend, Direction, Layout, Rect},
    Frame, Terminal,
};
use simbelmyne_chess::{board::Board, movegen::moves::Move};

use crate::engine::PerftThread;

use super::{
    board_view::BoardView,
    diff_table::DiffTable,
    engine::{Engine, Simbelmyne},
    info_view::InfoView,
};

pub type PerftResult = Vec<(Move, usize)>;

#[derive(Debug, Clone)]
pub struct Diff {
    pub mv: Move,
    pub found: Option<usize>,
    pub expected: Option<usize>,
}

pub struct State {
    engine: PerftThread,
    simbelmyne: PerftThread,
    expected: Arc<Mutex<PerftResult>>,
    found: Arc<Mutex<PerftResult>>,
    diffs: Vec<Diff>,
    selected: usize,
    depth: usize,
    initial_board: Board,
    board_stack: Vec<Board>,
    should_quit: bool,
}

impl State {
    fn new(depth: usize, fen: String, engine: PathBuf) -> State {
        let initial_board = fen.parse().unwrap();
        let simbelmyne = PerftThread::new(Simbelmyne {});
        let engine = PerftThread::new(Engine::new(engine).unwrap());

        Self {
            engine,
            simbelmyne,
            expected: Arc::new(Mutex::new(Vec::new())),
            found: Arc::new(Mutex::new(Vec::new())),
            diffs: vec![],
            selected: 0,
            depth,
            initial_board,
            board_stack: vec![initial_board],
            should_quit: false,
        }
    }

    fn run_perft(&mut self) {
        let board = self.board_stack.last().unwrap();
        let current_depth = self.board_stack.len();
        let remaining_depth = self.depth.saturating_sub(current_depth);

        self.engine.run(*board, remaining_depth, self.found.clone());
        self.simbelmyne.run(*board, remaining_depth, self.expected.clone());
    }

    fn refresh_diff(&mut self) -> anyhow::Result<()> {
        let mut results: BTreeMap<String, Diff> = BTreeMap::new();

        let expected = self.found.lock().unwrap();
        let found = self.expected.lock().unwrap();

        // Insert all of our moves, keyed by their algebraic string
        for (mv, count) in found.iter() {
            results.insert(
                mv.to_string(),
                Diff {
                    mv: *mv,
                    found: Some(*count),
                    expected: None,
                },
            );
        }

        // Fill in any moves that Simbelmyne also found
        for (mv, count) in expected.iter() {
            let diff = results.entry(mv.to_string()).or_insert(Diff {
                mv: *mv,
                found: None,
                expected: None,
            });

            diff.expected = Some(*count);
        }

        self.diffs = results.into_values().collect();
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Message {
    Up,
    Down,
    Select,
    Back,
    Quit,
}

fn view(state: &mut State, f: &mut Frame) {
    let term_rect = f.size();
    let layout = create_layout(term_rect);
    let current_board = state.board_stack.last().unwrap();

    let move_table = DiffTable {
        diffs: state.diffs.clone(),
        selected: state.selected,
    };

    let board_view = BoardView {
        board: *current_board,
    };

    let info_view = InfoView {
        starting_pos: state.initial_board.to_fen(),
        current_pos: current_board.to_fen(),
        search_depth: state.depth,
        current_depth: state.board_stack.len() - 1,
        total_found: state.diffs.iter().map(|d| d.found.unwrap_or(0)).sum(),
        total_expected: state.diffs.iter().map(|d| d.expected.unwrap_or(0)).sum(),
    };

    f.render_widget(move_table, layout.table);
    f.render_widget(board_view, layout.board);
    f.render_widget(info_view, layout.info);
}

struct LayoutChunks {
    table: Rect,
    board: Rect,
    info: Rect,
}

fn create_layout(container: Rect) -> LayoutChunks {
    let app_width = 130;
    let app_height = 42;

    let centered_rect = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min((container.height - app_height) / 2),
            Constraint::Min(app_height),
            Constraint::Min((container.height - app_height) / 2),
        ])
        .split(container)[1];

    let centered_rect = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min((container.width - app_width) / 2),
            Constraint::Min(app_width),
            Constraint::Min((container.width - app_width) / 2),
        ])
        .split(centered_rect)[1];

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(32), Constraint::Min(10)])
        .split(centered_rect);

    let top_panel = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Min(50)])
        .split(sections[0]);

    let bottom_panel = sections[1];

    let table_panel = top_panel[0];
    let board_panel = top_panel[1];

    LayoutChunks {
        table: table_panel,
        board: board_panel,
        info: bottom_panel,
    }
}

fn initialize_panic_handler() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen).unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();
        original_hook(panic_info);
    }));
}

fn handle_event(_: &State) -> anyhow::Result<Option<Message>> {
    let message = if crossterm::event::poll(std::time::Duration::from_millis(16))? {
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            match key.code {
                KeyCode::Char('j') => Message::Down,
                KeyCode::Char('k') => Message::Up,
                KeyCode::Char('q') | KeyCode::Esc => Message::Quit,
                KeyCode::Char('h') => Message::Back,
                KeyCode::Char('l') | KeyCode::Enter => Message::Select,
                _ => return Ok(None),
            }
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    Ok(Some(message))
}

fn update(state: &mut State, message: Message) -> Option<Message> {
    match message {
        Message::Up => {
            if 0 < state.selected {
                state.selected -= 1
            }
        }

        Message::Down => {
            if state.selected < state.diffs.len() - 1 {
                state.selected += 1
            }
        }

        Message::Quit => state.should_quit = true,

        Message::Select => {
            let current_depth = state.board_stack.len();

            if current_depth == state.depth {
                return None;
            }

            let current_board = state.board_stack.last().unwrap();
            let selected_move = state.diffs[state.selected].mv;

            let new_board = current_board.play_move(selected_move);

            state.board_stack.push(new_board);
            state.run_perft();
            state.refresh_diff().unwrap();
            state.selected = 0;
        }

        Message::Back => {
            let current_depth = state.board_stack.len();
            if current_depth == 1 {
                return None;
            }

            state.board_stack.pop();
            state.run_perft();
            state.refresh_diff().unwrap();
            state.selected = 0;
        }
    }

    None
}

pub fn init_tui(depth: usize, fen: String, engine: PathBuf) -> anyhow::Result<()> {
    initialize_panic_handler();

    // Startup
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut state = State::new(depth, fen, engine);
    state.run_perft();

    loop {
        state.refresh_diff().unwrap();

        // Render the current view
        terminal.draw(|f| {
            view(&mut state, f);
        })?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&state)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut state, current_msg.unwrap());
        }

        // Exit loop if quit flag is set
        if state.should_quit {
            break;
        }
    }

    // Shutdown
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
