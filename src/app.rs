use crossterm::event::{self, Event, KeyCode};
use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader},
    fs::File,
    error::Error
};
use tui::{
    backend::Backend,
    layout::Rect,
    Terminal
};
use crate::view::draw_ui;

pub struct App {
    pub logs: Vec<String>,
    pub num_logs: usize,
    pub sudo_logs: Vec<String>,
    pub commands: HashMap<String, usize>,
    pub titles: Vec<String>,
    pub tab_index: usize,
    pub page_index: usize,
    pub num_pages: usize,
    pub logs_per_page: usize,
}

impl App {
    pub fn new() -> App {
        let mut commands = HashMap::new();
        let (logs, sudo_logs) = load_logs(&mut commands).unwrap();
        App {
            logs,
            num_logs: 0,
            sudo_logs,
            commands,
            titles: vec!["ALL".to_string(), "SUDO".to_string(), "COMMANDS".to_string()],
            tab_index: 0,
            page_index: 0,
            num_pages: 0,
            logs_per_page: 0
        }
    }

    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        let mut set_start_page = true;

        loop {
            terminal.draw(|f| {
                let size = f.size();
                /* calculates necessary information, including:
                 * logs_per_page: the number of logs than can be displayed per page
                 * num_logs:      the total number of logs
                 * num_pages:     the total number of pages 
                 */
                update_log_information(&mut self, &size);

                if set_start_page {
                    self.page_index = self.num_pages - 1;
                    set_start_page = false;
                }

                // draws the ui
                draw_ui(f, &self, &size);
            })?;

            // handles all key inputs
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => {
                        if self.page_index != 0 {
                            self.page_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.page_index != self.num_pages - 1 {
                            self.page_index += 1;
                        }
                    }
                    KeyCode::Right => {
                        self.next();
                        set_start_page = true;
                    },
                    KeyCode::Left => {
                        self.prev();
                        set_start_page = true;
                    },
                    _ => {}
                }
            }
        }
    }

    pub fn next(&mut self) {
        self.tab_index = (self.tab_index + 1) % self.titles.len();
    }

    pub fn prev(&mut self) {
        if self.tab_index > 0 {
            self.tab_index -= 1;
        } else {
            self.tab_index = self.titles.len() - 1;
        }
    }
}

fn load_logs(commands: &mut HashMap<String, usize>) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {
    // open the auth.log file
    let file = File::open("/var/log/auth.log")?;
    let reader = BufReader::new(file);

    // vector to store the log entries
    let mut logs = Vec::new();
    let mut sudo_logs = Vec::new();

    // read the file line by line
    for line in reader.lines() {
        // unwrap the line or handle any potential error
        let line = line?;

        // if the log is sudo-related, parse the line and store the command used
        if line.contains("sudo:") && !line.contains("pam_unix") {
            let command_text = "COMMAND=";
            let command_index = line.find(command_text).unwrap();
            let (_, command )= line.split_at(command_index + command_text.len());
            if commands.contains_key(command) {
                commands.insert(command.to_string(), commands.get(command).unwrap() + 1);
            } else {
                commands.insert(command.to_string(), 1);
            }
            sudo_logs.push(line.clone());
        }

        // add the entries to the logs vector
        logs.push(line);
    }

    Ok((logs, sudo_logs))
}

fn update_log_information(app: &mut App, size: &Rect) {
    app.logs_per_page = size.height.into();

    app.num_logs = match app.tab_index {
        0 => app.logs.len(),
        1 => app.sudo_logs.len(),
        2 => app.sudo_logs.len(),
        _ => unreachable!()
    };

    if app.num_logs % app.logs_per_page == 0 {
        app.num_pages = app.num_logs / app.logs_per_page;
    } else {
        app.num_pages = (app.num_logs / app.logs_per_page) + 1;
    }
}