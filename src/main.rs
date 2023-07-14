use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, BufRead, BufReader},
    fs::File,
    error::Error
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Span, Spans},
    widgets::{Block, Wrap, Borders, Paragraph, Tabs},
    Frame, 
    Terminal
};

struct App {
    logs: Vec<Vec<String>>,
    num_logs: usize,
    titles: Vec<String>,
    tab_index: usize,
    page_index: usize,
    num_pages: usize,
    logs_per_page: usize,
}

impl App {
    fn new() -> App {
        App {
            logs: get_logs().unwrap(),
            num_logs: 0,
            titles: vec!["ALL".to_string(), "SUDO".to_string(), "SU".to_string(), "CRON".to_string(), "MISC".to_string()],
            tab_index: 0,
            page_index: 0,
            num_pages: 0,
            logs_per_page: 0
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

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut set_start_page = true;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            update_logs(app, &size);

            if set_start_page {
                app.page_index = app.num_pages - 1;
                set_start_page = false;
            }

            draw_ui(f, &size, app);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Up => {
                    if app.page_index != 0 {
                        app.page_index -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.page_index != app.num_pages - 1 {
                        app.page_index += 1;
                    }
                }
                KeyCode::Right => app.next(),
                KeyCode::Left => app.prev(),
                _ => {}
            }
        }
    }
}

fn update_logs(app: &mut App, size: &Rect) {
    app.logs_per_page = size.height.into();
    app.num_logs = app.logs.len();
    if app.num_logs % app.logs_per_page == 0 {
        app.num_pages = app.num_logs / app.logs_per_page;
    } else {
        app.num_pages = (app.num_logs / app.logs_per_page) + 1;
    }
}

fn draw_ui<B: Backend>(f: &mut Frame<B>, size: &Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(*size);

    let block = Block::default()
        .title("Terminal Interface for Sudo")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(200,200,200)));
    f.render_widget(block, *size);

    let titles = app.titles.iter().map(|t| {
        Spans::from(vec![
            Span::styled(t, Style::default().fg(Color::Rgb(120,120,120)))
        ])
    }).collect();

    let tabs = Tabs::new(titles)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
            "Tabs",
            Style::default().fg(Color::Rgb(217,111,13))
        )))
        .select(app.tab_index)
        .style(Style::default().fg(Color::Rgb(120,120,120)))
        .highlight_style(
            Style::default()
                .fg(Color::Rgb(200,200,200))
                .add_modifier(Modifier::BOLD)
        );
    f.render_widget(tabs, chunks[0]);

    // draw logs
    let text = calculate_subset(app);

    let mut text_spanss: Vec<Spans> = Vec::new();
    for log in text {
        let line = log.join(" ");
        if line.contains("sudo") {
            let sudo_index = line.find("sudo").unwrap();
            let (first, rest) = line.split_at(sudo_index);
            let (sudo, rest) = rest.split_at(4);
            text_spanss.push(Spans::from(vec![
                Span::styled(first.to_string(), Style::default().fg(Color::Rgb(200,200,200))),
                Span::styled(sudo.to_string(), Style::default().fg(Color::Red)),
                Span::styled(rest.to_string(), Style::default().fg(Color::Rgb(200,200,200))),
            ]));
        }
        else {
            text_spanss.push(Spans::from(vec![
                Span::styled(line, Style::default().fg(Color::Rgb(200,200,200)))
            ]))
        }
    }
    /*
    let text_spans: Vec<Spans> = text.iter().map(|log| {
        let text = log.join(" ");
        let span = Span::from(Span::styled(
            text,
            Style::default().fg(Color::Rgb(200,200,200))
        ));
        Spans::from(span)
    }).collect();*/

    let log_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            "Logs",
            Style::default().fg(Color::Rgb(217,111,13))
        ))
        .style(Style::default().fg(Color::Rgb(120,120,120)));
    
    let paragraph = Paragraph::new(text_spanss)
        .block(log_block)
        .wrap(Wrap{trim: true})
        .alignment(Alignment::Left);
    f.render_widget(paragraph, chunks[1]);
}

fn calculate_subset(app: &App) -> &[Vec<String>] {
    let first_log = app.page_index * app.logs_per_page;
    let last_log;
    if app.page_index == app.num_pages - 1 {
        last_log = app.num_logs - 1;
    } else {
        last_log = first_log + app.logs_per_page - 1;
    }
    let slice = &app.logs[first_log..last_log];
    slice
}

/*fn ui<B: Backend>(f: &mut Frame<B>) {
    let size = f.size();

    // Surrounding block
    let block = Block::default()
        .borders(Borders::ALL)
        .title("tuisulog")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Gray))
        .border_type(BorderType::Rounded);
    f.render_widget(block, size);

    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());

    let first_log = current_page * visible_lines;
    let last_log;
    if *current_page == num_pages - 1 {
        last_log = num_logs - 1;
    } else {
        last_log = first_log + visible_lines - 1;
    }
    let logs_on_page = &logs[first_log..last_log];

    let text_spans: Vec<Spans> = logs_on_page.iter().map(|log| {
        let text = log.join(" ");
        let span = Span::raw(text);
        Spans::from(span)
    }).collect();

    f.render_widget(Paragraph::new(text_spans), chunks[0]);

    /*
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(4)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    // Top two inner blocks
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    // Top left inner block with green background
    let block = Block::default()
        .title(vec![
            Span::styled("With", Style::default().fg(Color::Yellow)),
            Span::from(" background"),
        ])
        .style(Style::default().bg(Color::Green));
    f.render_widget(block, top_chunks[0]);

    // Top right inner block with styled title aligned to the right
    let block = Block::default()
        .title(Span::styled(
            "Styled title",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Right);
    f.render_widget(block, top_chunks[1]);

    // Bottom two inner blocks
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    // Bottom left block with all default borders
    let block = Block::default().title("With borders").borders(Borders::ALL);
    f.render_widget(block, bottom_chunks[0]);

    // Bottom right block with styled left and right border
    let block = Block::default()
        .title("With styled borders and doubled borders")
        .border_style(Style::default().fg(Color::Cyan))
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_type(BorderType::Double);
    f.render_widget(block, bottom_chunks[1]);
    */
}*/

fn get_logs() -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    // Open the auth.log file
    //let file = File::open("/var/log/auth.log")?;
    let file = File::open("/home/ehelwig/tuisulog/text.log")?;
    let reader = BufReader::new(file);

    // Vector to store the log entries
    let mut logs = Vec::new();

    // Read the file line by line
    for line in reader.lines() {
        // Unwrap the line or handle any potential error
        let line = line?;

        // Split the line by whitespace
        let entries = line.split_whitespace().map(|s| s.to_string()).collect();

        // Add the entries to the logs vector
        logs.push(entries);
    }

    Ok(logs)
}