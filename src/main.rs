mod encryption;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use encryption::encryption::{decrypt_data, encrypt_data, reset_file_cursor};
use orion::{aead, aead::SecretKey};
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, io::Stdout};
use std::{fs::OpenOptions, str::from_utf8};
use std::{io, process::exit};
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

#[derive(Serialize, Deserialize, Clone)]
struct Password {
    domain: String,
    username: String,
    password: String,
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

// struct for managing overall app state
#[derive(Default)]
struct AppState {
    config_path: String,
    store_path: String,
    secret_key: aead::SecretKey,
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

fn create_windows_config(store_path: &String, config_dir: &String, secret_key: &SecretKey) {

    fs::create_dir_all(&config_dir).unwrap();
         let mut store = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&store_path)
            .unwrap();

    store
        .write(b"[{\"domain\": \"\", \"username\": \"\", \"password\": \"\" }]")
        .unwrap();

    encrypt_data(&mut store, &secret_key);

}

fn create_unix_config(store_path: &String, config_dir: &String, secret_key: &SecretKey) {
    Command::new("mkdir")
            .arg(config_dir)
            .output()
            .expect("Error making .arustylock directory");
        let mut store = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&store_path)
            .unwrap();

        store
            .write(b"[{\"domain\": \"\", \"username\": \"\", \"password\": \"\" }]")
            .unwrap();

        encrypt_data(&mut store, &secret_key);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode().expect("Can't run in raw mode");

    let mut app = AppState::default();

    // Adds in a newline char here
    // so we want to truncate it
    let mut user = Command::new("whoami")
        .output()
        .expect("Error finding out current user")
        .stdout;
    let len = user.len();
    user.truncate(len - 1);
    let config_dir: String;

    if cfg!(windows) {

        config_dir = format!(
           "C:\\Users\\{}\\AppData\\Roaming\\arustylock",
            String::from_utf8(user).expect("Error reading stdout to string")
        );

    } else {
        config_dir = format!(
            "/home/{}/.config/arustylock",
            String::from_utf8(user).expect("Error reading stdout to string")
        );
    }


    let store_path = format!("{}/data", config_dir);

    // Can't use the default since it randomly generates a key each time
    // Might need to entirely redo how we encrypt if we actually want security lol
    // Perhaps another day...
    let secret_key = SecretKey::from_slice("qaz123WSX$%^edcplm098IJN765uhbZQ".as_bytes()).unwrap();

    if !Path::new(config_dir.as_str()).exists() {
        if cfg!(windows) {
            create_windows_config(&store_path, &config_dir, &secret_key);
        } else {
            create_unix_config(&store_path, &config_dir, &secret_key);
        }
    }

    app.config_path = config_dir;
    app.store_path = store_path;
    app.secret_key = secret_key;
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen");
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

            if event::poll(timeout).expect("poll failed") {
                if let CEvent::Key(key) = event::read().expect("Couldn't read event") {
                    tx.send(Event::Input(key)).expect("Couldn't send events");
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

            let copyright = Paragraph::new("A Rusty Lock - all rights reserved")
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
                    let (left, right) = render_passwords(&password_list_state, &mut app);
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
                handle_home_keyevent(&received, &mut active_menu_item, &mut terminal, &mut app);
            }
            MenuItem::Passwords => {
                handle_passwords_keyevent(
                    &received,
                    &mut active_menu_item,
                    &mut password_list_state,
                    &mut app,
                    &mut terminal,
                );
            }
            MenuItem::AddPassword => {
                handle_add_keyevent(
                    &received,
                    &mut active_menu_item,
                    &mut add_password_state,
                    &mut app,
                    &mut terminal,
                );
            }
        }
    }
}

fn handle_home_keyevent(
    key_event: &Event<KeyEvent>,
    active_menu_item: &mut MenuItem,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
) {
    match key_event {
        Event::Input(event) => match event.code {
            KeyCode::Char('q') => {
                disable_raw_mode().expect("Raw mode was not disabled");

                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )
                .expect("Leaving alt screen failed");
                terminal.show_cursor().expect("Unable to show cursor");
                exit(0);
            }
            KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
            KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
            KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,

            _ => {}
        },
        Event::Tick => {}
    }
}

fn handle_passwords_keyevent(
    key_event: &Event<KeyEvent>,
    active_menu_item: &mut MenuItem,
    password_list_state: &mut ListState,
    app: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) {
    match key_event {
        Event::Input(event) => match event.code {
            KeyCode::Char('d') => {
                remove_password_at_index(password_list_state, app)
                    .expect("Couldn't remove password");
            }
            KeyCode::Char('q') => {
                disable_raw_mode().expect("Raw mode was not disabled");

                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )
                .expect("Leaving alt screen failed");
                terminal.show_cursor().expect("Unable to show cursor");
                exit(0);
            }
            KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
            KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
            KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,
            KeyCode::Char('j') => {
                if let Some(selected) = password_list_state.selected() {
                    let amount_passwords = read_db(app).expect("Couldn't fetch passwords").len();
                    if selected >= amount_passwords - 1 {
                        password_list_state.select(Some(0));
                    } else {
                        password_list_state.select(Some(selected + 1));
                    }
                }
            }
            KeyCode::Char('k') => {
                if let Some(selected) = password_list_state.selected() {
                    let amount_passwords = read_db(app).expect("can fetch password list").len();
                    if selected <= 0 {
                        password_list_state.select(Some(amount_passwords - 1));
                    } else {
                        password_list_state.select(Some(selected - 1));
                    }
                }
            }

            _ => {}
        },
        Event::Tick => {}
    }
}

fn handle_add_keyevent(
    key_event: &Event<KeyEvent>,
    active_menu_item: &mut MenuItem,
    input_state: &mut InputState,
    app: &mut AppState,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) {
    match input_state.input_mode {
        InputMode::DomainNormal => match key_event {
            Event::Input(event) => match event.code {
                KeyCode::Char('i') => input_state.input_mode = InputMode::DomainEditing,
                KeyCode::Char('j') => input_state.input_mode = InputMode::UsernameNormal,
                KeyCode::Char('h') => *active_menu_item = MenuItem::Home,
                KeyCode::Char('p') => *active_menu_item = MenuItem::Passwords,
                KeyCode::Char('a') => *active_menu_item = MenuItem::AddPassword,
                KeyCode::Char('q') => {
                    disable_raw_mode().expect("Raw mode was not disabled");

                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )
                    .expect("Leaving alt screen failed");
                    terminal.show_cursor().expect("Unable to show cursor");
                    exit(0)
                }

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
                KeyCode::Char('q') => {
                    disable_raw_mode().expect("Raw mode was not disabled");

                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )
                    .expect("Leaving alt screen failed");
                    terminal.show_cursor().expect("Unable to show cursor");
                    exit(0);
                }
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
                KeyCode::Char('q') => {
                    disable_raw_mode().expect("Raw mode was not disabled");

                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )
                    .expect("Leaving alt screen failed");
                    terminal.show_cursor().expect("Unable to show cursor");
                    exit(0);
                }

                KeyCode::Enter => {
                    add_password_to_db(input_state, app).expect("Failed to add password");
                    clear_input(input_state);
                }
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
                KeyCode::Enter => {
                    add_password_to_db(input_state, app).expect("Failed to add password");
                    clear_input(input_state);
                }
                _ => {}
            },
            Event::Tick => {}
        },
    }
}

fn clear_input(input_state: &mut InputState) {
    input_state.input_domain = String::new();
    input_state.input_username = String::new();
    input_state.input_password = String::new();
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "A Rusty Lock - The \"\"\"best\"\"\" password manager",
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

fn render_passwords<'a>(
    password_list_state: &ListState,
    app: &mut AppState,
) -> (List<'a>, Table<'a>) {
    let passwords = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Passwords")
        .border_type(BorderType::Plain);

    let password_list = read_db(app).expect("Couldn't fetch passwords list");
    let items: Vec<_> = password_list
        .iter()
        .map(|password| {
            ListItem::new(Spans::from(vec![Span::styled(
                password.domain.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_password = password_list
        .get(
            password_list_state
                .selected()
                .expect("Couldn't get selected password"),
        )
        .expect("Error getting selected password")
        .clone();

    let list = List::new(items).block(passwords).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let password_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_password.domain)),
        Cell::from(Span::raw(selected_password.username)),
        Cell::from(Span::raw(selected_password.password)),
    ])])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            "Domain",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Username",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Password",
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
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ]);

    (list, password_detail)
}

fn render_create_password<'a>(
    input_state: &'a InputState,
) -> (Paragraph<'a>, Paragraph<'a>, Paragraph<'a>) {
    let domain_input = Paragraph::new(input_state.input_domain.as_ref())
        .style(match input_state.input_mode {
            InputMode::DomainNormal => Style::default().fg(Color::Yellow),
            InputMode::DomainEditing => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::White),
        })
        .block(Block::default().borders(Borders::ALL).title("Domain"));

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

    return (domain_input, username_input, password_input);
}

fn read_db(app: &mut AppState) -> Result<Vec<Password>, Error> {
    let mut store = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&app.store_path)
        .unwrap();
    let data = decrypt_data(&mut store, &app.secret_key);
    let parsed: Vec<Password> = serde_json::from_str(from_utf8(&data).unwrap())?;
    Ok(parsed)
}

fn add_password_to_db(
    input_state: &InputState,
    app: &mut AppState,
) -> Result<Vec<Password>, Error> {
    let mut store = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&app.store_path)
        .unwrap();
    let data = decrypt_data(&mut store, &app.secret_key);
    let mut parsed: Vec<Password> = serde_json::from_str(from_utf8(&data).unwrap())?;
    let new_password = Password {
        domain: input_state.input_domain.clone(),
        username: input_state.input_username.clone(),
        password: input_state.input_password.clone(),
    };
    parsed.push(new_password);
    // Convert this into Vec<u8> and then encrypt it and then write it to the store
    let json_string: String = serde_json::to_string(&parsed).unwrap();
    let slice: &[u8] = json_string.as_bytes();
    let cipher_text = aead::seal(&app.secret_key, &slice).unwrap();

    // fs::write(&app.store_path, &serde_json::to_vec(&parsed)?)?;
    reset_file_cursor(&mut store);
    store.set_len(0).unwrap();
    store.write_all(&cipher_text).unwrap();
    // File is already encrypted at this point, so it's redundant
    Ok(parsed)
}

fn remove_password_at_index(
    password_list_state: &mut ListState,
    app: &mut AppState,
) -> Result<(), Error> {
    // This is a workaround to prevent the program from crashing after removing
    // the last password
    let password_list = read_db(app).expect("can fetch password list");
    if password_list.len() > 1 {
        if let Some(selected) = password_list_state.selected() {
            let mut store = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&app.store_path)
                .unwrap();
            let data = decrypt_data(&mut store, &app.secret_key);
            let mut parsed: Vec<Password> = serde_json::from_str(from_utf8(&data).unwrap())?;

            parsed.remove(selected);

            let json_string: String = serde_json::to_string(&parsed).unwrap();
            let slice: &[u8] = json_string.as_bytes();
            let cipher_text = aead::seal(&app.secret_key, &slice).unwrap();

            //fs::write(&app.store_path, &serde_json::to_vec(&parsed)?)?;
            reset_file_cursor(&mut store);
            store.set_len(0).unwrap();
            store.write_all(&cipher_text).unwrap();
            if selected > 0 {
                password_list_state.select(Some(selected - 1));
            }
        }
    }
    Ok(())
}
