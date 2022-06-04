use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, io::Stdout};
use thiserror::Error;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};

const DB_PATH: &str = "./data/db.json";

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

// #[derive(Serialize, Deserialize, Clone)]
// struct Pet {
//     id: usize,
//     name: String,
//     category: String,
//     age: usize,
//     created_at: DateTime<Utc>,
// }
// Password struct that will take the place of Pet
#[derive(Serialize, Deserialize, Clone)]
struct Password {
    id: usize,
    domain: String,
    username: String,
    password: String,
    created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Passwords,
    AddPassword,
}

enum InputMode {
    DomainEditing,
    DomainNormal,
    UsernameEditing,
    UsernameNormal,
    PasswordEditing,
    PasswordNormal,
}

impl Default for InputMode {
    fn default() -> Self {
        InputMode::DomainNormal
    }
}

// struct for managing state in adding new credentials
#[derive(Default)]
struct InputState {
    input_domain: String,
    input_username: String,
    input_password: String,
    input_mode: InputMode,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Passwords => 1,
            MenuItem::AddPassword => 2,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("can run in raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Home", "Passwords", "Add", "Delete", "Quit"];
    let mut active_menu_item = MenuItem::Home;
    let mut password_list_state = ListState::default();
    let mut add_password_state = InputState::default();
    password_list_state.select(Some(0));

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    loop {
        terminal.draw(|rect| {
            let size = rect.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let copyright = Paragraph::new("RustLock - all rights reserved")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Copyright")
                        .border_type(BorderType::Plain),
                );

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(active_menu_item.into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            rect.render_widget(tabs, chunks[0]);
            match active_menu_item {
                MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
                MenuItem::Passwords => {
                    let passwords_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
                        )
                        .split(chunks[1]);
                    let (left, right) = render_passwords(&password_list_state);
                    rect.render_stateful_widget(
                        left,
                        passwords_chunks[0],
                        &mut password_list_state,
                    );
                    rect.render_widget(right, passwords_chunks[1]);
                }
                MenuItem::AddPassword => {
                    let add_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Percentage(33),
                                Constraint::Percentage(33),
                                Constraint::Percentage(33),
                            ]
                            .as_ref(),
                        )
                        .split(chunks[1]);
                    let (top, center, bottom) = render_create_password(&add_password_state);
                    rect.render_widget(top, add_layout[0]);
                    rect.render_widget(center, add_layout[1]);
                    rect.render_widget(bottom, add_layout[2]);
                }
            }
            rect.render_widget(copyright, chunks[2]);
        })?;
        let received = rx.recv().unwrap();
        match active_menu_item {
            MenuItem::Home => {
                handle_home_keyevent(&received, &mut active_menu_item, &mut terminal);
            }
            MenuItem::Passwords => {
                handle_passwords_keyevent(&received, &mut active_menu_item);
            }
            MenuItem::AddPassword => {
                handle_add_keyevent(&received, &mut active_menu_item, &mut add_password_state);
            }
        }
    }
}

fn handle_home_keyevent(
    key_event: &Event<KeyEvent>,
    active_menu_item: &mut MenuItem,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) {
    match key_event {
        Event::Input(event) => match event.code {
            KeyCode::Char('q') => {
                // TODO: Command prompt not showing up properly after quiting
                disable_raw_mode().expect("Disabling raw mode...");
                terminal.show_cursor().expect("Showing cursor...");
                return;
            }
            KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
            KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
            KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,

            _ => {}
        },
        Event::Tick => {}
    }
}

fn handle_passwords_keyevent(key_event: &Event<KeyEvent>, active_menu_item: &mut MenuItem) {
    match key_event {
        Event::Input(event) => match event.code {
            KeyCode::Char('d') => {
                // delete currrently highlighted credentials
            }
            KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
            KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
            KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,
            _ => {}
        },
        Event::Tick => {}
    }
}

fn handle_add_keyevent(
    key_event: &Event<KeyEvent>,
    active_menu_item: &mut MenuItem,
    input_state: &mut InputState,
) {
    match input_state.input_mode {
        InputMode::DomainNormal => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Char('i') => input_state.input_mode = InputMode::DomainEditing,
                KeyCode::Char('j') => input_state.input_mode = InputMode::UsernameNormal,
                KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
                KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
                KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,
                _ => {}
            },
            Event::Tick => {}
        },
        InputMode::DomainEditing => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Esc => input_state.input_mode = InputMode::DomainNormal,
                KeyCode::Char(c) => input_state.input_domain.push(c),
                KeyCode::Backspace => {
                    input_state.input_domain.pop();
                }
                _ => {}
            },
            Event::Tick => {}
        },
        InputMode::UsernameNormal => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Char('i') => input_state.input_mode = InputMode::UsernameEditing,
                KeyCode::Char('j') => input_state.input_mode = InputMode::PasswordNormal,
                KeyCode::Char('k') => input_state.input_mode = InputMode::DomainNormal,
                KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
                KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
                KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,
                _ => {}
            },
            Event::Tick => {}
        },
        InputMode::UsernameEditing => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Esc => input_state.input_mode = InputMode::UsernameNormal,
                KeyCode::Char(c) => input_state.input_username.push(c),
                KeyCode::Backspace => {
                    input_state.input_username.pop();
                }
                _ => {}
            },
            Event::Tick => {}
        },
        InputMode::PasswordNormal => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Char('i') => input_state.input_mode = InputMode::PasswordEditing,
                KeyCode::Char('k') => input_state.input_mode = InputMode::UsernameNormal,
                KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
                KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
                KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,
                KeyCode::Enter => {}
                _ => {}
            },
            Event::Tick => {}
        },
        InputMode::PasswordEditing => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Esc => input_state.input_mode = InputMode::PasswordNormal,
                KeyCode::Char(c) => input_state.input_password.push(c),
                KeyCode::Backspace => {
                    input_state.input_password.pop();
                }
                KeyCode::Enter => {}
                _ => {}
            },
            Event::Tick => {}
        },
    }
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "RustLock",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'p' to access passwords, 'a' to add a new password and 'd' to delete the currently selected password.")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );
    home
}

fn render_passwords<'a>(password_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let passwords = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Passwords")
        .border_type(BorderType::Plain);

    let password_list = read_db().expect("can fetch password list");
    let items: Vec<_> = password_list
        .iter()
        .map(|password| {
            ListItem::new(Spans::from(vec![Span::styled(
                password.username.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_password = password_list
        .get(
            password_list_state
                .selected()
                .expect("there is always a selected password"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(passwords).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let password_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_password.id.to_string())),
        Cell::from(Span::raw(selected_password.username)),
        Cell::from(Span::raw(selected_password.password)),
        Cell::from(Span::raw(selected_password.created_at.to_string())),
    ])])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            "ID",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Name",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Category",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Age",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Created At",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Detail")
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(5),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(5),
        Constraint::Percentage(20),
    ]);

    (list, password_detail)
}

fn render_create_password<'a>(
    input_state: &'a InputState,
) -> (Paragraph<'a>, Paragraph<'a>, Paragraph<'a>) {
    // let domain_input = Block::default()
    //     .borders(Borders::ALL)
    //     .style(match input_state.input_mode {
    //         InputMode::DomainNormal => Style::default().fg(Color::Yellow),
    //         InputMode::DomainEditing => Style::default().fg(Color::Green),
    //         _ => Style::default().fg(Color::White),
    //     })
    //     .title("New Domain")
    //     .border_type(BorderType::Plain);

    let domain_input = Paragraph::new(input_state.input_domain.as_ref())
        .style(match input_state.input_mode {
            InputMode::DomainNormal => Style::default().fg(Color::Yellow),
            InputMode::DomainEditing => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::White),
        })
        .block(Block::default().borders(Borders::ALL).title("Domain"));

    //   let username_input = Block::default()
    //       .borders(Borders::ALL)
    //       .style(match input_state.input_mode {
    //           InputMode::UsernameNormal => Style::default().fg(Color::Yellow),
    //           InputMode::UsernameEditing => Style::default().fg(Color::Green),
    //           _ => Style::default().fg(Color::White),
    //       })
    //       .title("New Username")
    //       .border_type(BorderType::Plain);

    let username_input = Paragraph::new(input_state.input_username.as_ref())
        .style(match input_state.input_mode {
            InputMode::UsernameNormal => Style::default().fg(Color::Yellow),
            InputMode::UsernameEditing => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::White),
        })
        .block(Block::default().borders(Borders::ALL).title("Username"));

    let password_input = Paragraph::new(input_state.input_password.as_ref())
        .style(match input_state.input_mode {
            InputMode::PasswordNormal => Style::default().fg(Color::Yellow),
            InputMode::PasswordEditing => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::White),
        })
        .block(Block::default().borders(Borders::ALL).title("Password"));

    //  let password_input = Block::default()
    //      .borders(Borders::ALL)
    //      .style(match input_state.input_mode {
    //          InputMode::PasswordNormal => Style::default().fg(Color::Yellow),
    //          InputMode::PasswordEditing => Style::default().fg(Color::Green),
    //          _ => Style::default().fg(Color::White),
    //      })
    //      .title("New Password")
    //      .border_type(BorderType::Plain);

    return (domain_input, username_input, password_input);
}

fn read_db() -> Result<Vec<Password>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<Password> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}

fn add_password_to_db(input_state: &InputState) -> Result<Vec<Password>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<Password> = serde_json::from_str(&db_content)?;

    for pat in &parsed {
        println!("{}", pat.domain);
    }

    fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
    Ok(parsed)
}

fn remove_password_at_index(password_list_state: &mut ListState) -> Result<(), Error> {
    // This is a workaround to prevent the program from crashing after removing
    // the last password
    let password_list = read_db().expect("can fetch password list");
    if password_list.len() > 1 {
        if let Some(selected) = password_list_state.selected() {
            let db_content = fs::read_to_string(DB_PATH)?;
            let mut parsed: Vec<Password> = serde_json::from_str(&db_content)?;
            parsed.remove(selected);
            fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;

            if selected > 0 {
                password_list_state.select(Some(selected - 1));
            }
        }
    }
    Ok(())
}
