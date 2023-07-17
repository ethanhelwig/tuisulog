use std::{
    io::{BufRead, BufReader},
    fs::File,
    error::Error
};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Span, Spans},
    widgets::{Block, Wrap, Borders, Paragraph, Tabs},
    Frame,
};
use crate::app::App;

pub fn draw_ui<B: Backend>(f: &mut Frame<B>, app: &App, size: &Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(*size);

    // render title
    let title_block = Block::default()
        .title("Super User Management Interface")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(200,200,200)));
    f.render_widget(title_block, *size);

    // render tabs
    let tabs = create_tabs(&app.titles, app.tab_index);
    f.render_widget(tabs, layout[0]);

    // render logs
    let logs_paragraph = create_logs_paragraph(app);
    f.render_widget(logs_paragraph, layout[1]);

    // render info
    let info_block = Block::default();
    let info_text = String::from(format!("\"/var/log/auth.log\" page: {}/{} logs: {}  (use arrow keys to navigate, press q to exit) ", app.page_index + 1, app.num_pages, app.num_logs));
    let text_spans = Spans::from(
        Span::styled(info_text, Style::default().bg(Color::Rgb(200,200,200)).fg(Color::Black))
    );
    let paragraph = Paragraph::new(text_spans)
        .block(info_block)
        .alignment(Alignment::Left);
    f.render_widget(paragraph, layout[2]);
}

fn create_tabs(titles: &Vec<String>, tab_index: usize) -> Tabs {
    let titles = titles.iter().map(|t| {
        Spans::from(vec![
            Span::styled(t.to_string(), Style::default().fg(Color::Rgb(120,120,120)))
        ])
    }).collect();

    Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            "Tabs",
            Style::default().fg(Color::Rgb(217,111,13))
        )))
        .select(tab_index)
        .style(Style::default().fg(Color::Rgb(120,120,120)))
        .highlight_style(Style::default()
            .fg(Color::Rgb(200,200,200))
            .add_modifier(Modifier::BOLD)
    )
}

fn create_logs_paragraph(app: &App) -> Paragraph {
    let log_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled("Logs", Style::default().fg(Color::Rgb(217,111,13))))
        .style(Style::default().fg(Color::Rgb(120,120,120)));
    let page = get_page(app);
    let spans = create_spans(page);

    Paragraph::new(spans)
        .block(log_block)
        .wrap(Wrap{trim: true})
        .alignment(Alignment::Left)
}

fn create_spans(page: &[String]) -> Vec<Spans> {
    let sudoers = get_sudoers().unwrap();
    let mut text_spans: Vec<Spans> = Vec::new();

    for log in page {
        let mut spans = Vec::new();
        let sudo_text = "sudo";
        // parse through lines to color code "sudo" and super users for readability
        if log.contains(sudo_text) {
            let mut log_index = 0;
            let mut is_match = false;
            let mut word = String::new();
            for ch in log.chars() {
                word.push(ch);
                let mut could_match = false;

                if sudo_text == word {
                    is_match = true;
                    // seperates the line into sections
                    let rest = &log[log_index..];
                    let index = rest.find(sudo_text).unwrap();
                    let (front, _) = rest.split_at(index);
                    log_index += front.len() + sudo_text.len();
                    // add sections that are ready
                    spans.push(Span::styled(front.to_string(), Style::default().fg(Color::Rgb(120,120,120))));
                    spans.push(Span::styled(sudo_text.to_string(), Style::default().fg(Color::LightRed)));
                } 
                else if sudo_text.starts_with(&word) {
                    could_match = true;
                }
                else {
                    for username in &sudoers {
                        // if the word built this far matches a username
                        if *username == word {
                            is_match = true;
                            let rest = &log[log_index..];
                            // parse it and push the completed sections
                            let index = rest.find(username).unwrap();
                            let (front, _) = rest.split_at(index);
                            log_index += front.len() + username.len();

                            if !front.is_empty() {
                                spans.push(Span::styled(front.to_string(), Style::default().fg(Color::Rgb(120,120,120))));
                            }
                            spans.push(Span::styled(username.to_string(), Style::default().fg(Color::White)));
                        } // but if it still matches the start of a username
                        else if username.starts_with(&word) {
                            could_match = true;
                        }
                    }
                }

                // if nothing matches or a username was found, clear the word
                if !could_match || is_match {
                    word.clear();
                    is_match = false;
                }
            }

            if !log[log_index..].is_empty() {
                spans.push(Span::styled(log[log_index..].to_string(), Style::default().fg(Color::Rgb(120,120,120))));
            }
        } else {
            spans.push(Span::styled(log.to_string(), Style::default().fg(Color::Rgb(120,120,120))));
        }

        text_spans.push(Spans::from(spans));
    }

    text_spans
}

fn get_sudoers() -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open("/etc/group")?;
    let reader = BufReader::new(file);

    let mut sudoers = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.contains("sudo") {
            let parsed_line: Vec<&str> = line.split_terminator(":").collect();
            let group_info_len = 3;
            if parsed_line.len() > group_info_len {
                let usernames: Vec<&str> = parsed_line[3].split_terminator(",").collect();
                for u in usernames {
                    sudoers.push(u.to_string());
                }
            }
        }
    }
    Ok(sudoers)
}

fn get_page(app: &App) -> &[String] {
    let first_log = app.page_index * app.logs_per_page;
    let last_log: usize;
    if app.page_index == app.num_pages - 1 {
        last_log = app.num_logs - 1;
    } else {
        last_log = first_log + app.logs_per_page - 1;
    }

    match app.tab_index {
        0 => &app.logs[first_log..last_log],
        1 => &app.sudo_logs[first_log..last_log],
        _ => unreachable!()
    }
}