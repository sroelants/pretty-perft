pub(crate) use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;

use crossterm::event::KeyCode;
use ratatui::prelude::Constraint;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::{
    prelude::{CrosstermBackend, Direction, Layout, Rect},
    Frame, Terminal,
};
use simbelmyne_chess::{board::Board, movegen::moves::Move};

use crate::components::centered;
use crate::engine::Engine;
use crate::engine::Executable;
use crate::engine::Simbelmyne;
use crate::engine::PerftThread;
use crate::Config;

use crate::components::{
    board_view::BoardView,
    diff_table::DiffTable,
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
    fn new(depth: usize, fen: String, engine: PerftThread) -> State {
        let initial_board = fen.parse().unwrap();
        let simbelmyne = PerftThread::new(Simbelmyne {});

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
    let term_rect = f.area();
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
        current_depth: state.board_stack.len(),
        total_found: state.diffs.iter().map(|d| d.found.unwrap_or(0)).sum(),
        total_expected: state.diffs.iter().map(|d| d.expected.unwrap_or(0)).sum(),
    };

    let help = Text::from(
        Line::from(vec![
            Span::styled("k ", Style::new().fg(Color::Blue)),
            Span::styled("Up, ", Style::new().fg(Color::DarkGray)),
            Span::styled("j ", Style::new().fg(Color::Blue)),
            Span::styled("Down, ", Style::new().fg(Color::DarkGray)),
            Span::styled("l ", Style::new().fg(Color::Blue)),
            Span::styled("Select, ", Style::new().fg(Color::DarkGray)),
            Span::styled("h ", Style::new().fg(Color::Blue)),
            Span::styled("Back, ", Style::new().fg(Color::DarkGray)),
            Span::styled("q ", Style::new().fg(Color::Blue)),
            Span::styled("Quit, ", Style::new().fg(Color::DarkGray)),
        ])
    );
        // format!(
        //     "{k} - {up}, {j} - {down}, {l} - {select}, {h} - {back}, {q} - {quit}",
        //     k = "k".blue(),
        //     up = "Up".grey(),
        //     j = "j".blue(),
        //     down = "Down".grey(),
        //     l = "l".blue(),
        //     select = "Select".grey(),
        //     h = "h".blue(),
        //     back = "Back".grey(),
        //     q = "q/Esc".blue(),
        //     quit = "Quit".grey(),
        // )
    // );


    f.render_widget(move_table, layout.table);
    f.render_widget(board_view, layout.board);
    f.render_widget(info_view, layout.info);
    f.render_widget(help, layout.help);
}

struct LayoutChunks {
    table: Rect,
    board: Rect,
    info: Rect,
    help: Rect,
}

fn create_layout(container: Rect) -> LayoutChunks {
    let app_width = 130;
    let app_height = 45;

    let centered_rect = centered(container, app_width, app_height);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(32), Constraint::Min(10), Constraint::Min(3)])
        .split(centered_rect);

    let top_panel = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Min(50)])
        .split(sections[0]);

    let bottom_panel = sections[1];
    let help_area = sections[2];

    let table_panel = top_panel[0];
    let board_panel = top_panel[1];

    LayoutChunks {
        table: table_panel,
        board: board_panel,
        info: bottom_panel,
        help: help_area,
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

            if current_depth + 1 == state.depth {
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

impl Config {
    pub fn run(&self) -> anyhow::Result<()> {
    initialize_panic_handler();

    // Startup
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let engine = if let Some(engine) = &self.engine {
            PerftThread::new(Engine::new(engine.to_path_buf()).unwrap())
        } else if let Some(command) = &self.command {
            PerftThread::new(Executable::new(command.to_path_buf()))
        } else {
            panic!()
        };

    let mut state = State::new(self.depth, self.fen.to_string(), engine);
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
}
