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

/// Renders the user interface for the Super User Management Interface.
///
/// The function is responsible for rendering the title, tabs, logs, and info paragraphs
/// based on the current tab. The rendered UI will be displayed using the provided `Frame`.
///
/// # Arguments
///
/// * `f`: A mutable reference to a `Frame`, which is used to render the UI elements.
/// * `app`: A reference to an `App` struct, containing the data for the user interface.
/// * `size`: A reference to a `Rect`, representing the available size to draw the UI.
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

    match app.tab_index {
        0 | 1 => {
            // render logs
            let logs_paragraph = create_logs_paragraph(app);
            f.render_widget(logs_paragraph, layout[1]);

            // render info
            let info_paragraph = create_info_paragraph(app);
            f.render_widget(info_paragraph, layout[2]);
        },
        2 => {
            let horiz_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(layout[1]);

            // render recent commands
            let recent_cmds_paragraph = create_recent_commands_paragraph(app);
            f.render_widget(recent_cmds_paragraph, horiz_layout[0]);

            // render most used commands chart
            let most_used_cmds_paragraph = create_most_used_cmds_paragraph(app);
            f.render_widget(most_used_cmds_paragraph, horiz_layout[1]);
        },
        _ => unreachable!()
    }
}

/// Creates and returns `Tabs` from a Vector of Strings. `Tabs` is a special
/// type of block for displaying Spans in a multi-panel context.
/// 
/// # Arguments
///
/// * `titles`: A reference to a `Vec<String>` object representing the tab titles.
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

fn create_info_paragraph(app: &App) -> Paragraph {
    let info_block = Block::default();
    let info_text = String::from(format!("\"/var/log/auth.log\" page: {}/{} logs: {}  (use arrow keys to navigate, press q to exit) ", app.page_index + 1, app.num_pages, app.num_logs));
    let text_spans = Spans::from(
        Span::styled(info_text, Style::default().bg(Color::Rgb(200,200,200)).fg(Color::Black))
    );

    Paragraph::new(text_spans)
        .block(info_block)
        .alignment(Alignment::Left)
}

fn create_recent_commands_paragraph(app: &App) -> Paragraph {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled("Recent", Style::default().fg(Color::Rgb(217,111,13))))
        .style(Style::default().fg(Color::Rgb(120,120,120)));

    let mut spans: Vec<Spans> = Vec::new();
    let cmd_text = "COMMAND=";
    let recent_logs_to_get = 10;
    let recent_log_index = app.num_logs - recent_logs_to_get;
    for log in &app.sudo_logs[recent_log_index..] {
        let mut span: Vec<Span> = Vec::new();
        // user
        let sudo_text = "sudo:";
        let delimiter = ":";
        let user_start_index = log.find(sudo_text).unwrap() + sudo_text.len();
        let user_end_index = log[user_start_index..].find(delimiter).unwrap() + user_start_index;
        let user = format!("{}: ", log[user_start_index..user_end_index].trim());
        span.push(Span::styled(user, Style::default().fg(Color::Rgb(120,120,120))));

        // command
        let cmd_index = log.find(cmd_text).unwrap();
        let (_, cmd) = log.split_at(cmd_index + cmd_text.len());
        span.push(Span::styled(cmd, Style::default().fg(Color::Rgb(120,120,120))));

        spans.push(Spans::from(span));
    }

    Paragraph::new(spans)
        .block(block)
        .wrap(Wrap{trim: true})
        .alignment(Alignment::Left)
}

fn create_most_used_cmds_paragraph(app: &App) -> Paragraph {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled("Frequency", Style::default().fg(Color::Rgb(217,111,13))))
        .style(Style::default().fg(Color::Rgb(120,120,120)));

    let mut spans: Vec<Spans> = Vec::new();

    // Extract commands and sort their frequencies in descending order
    let mut sorted_pairs: Vec<(&String, &usize)> = app.commands.iter().collect();
    sorted_pairs.sort_by(|a, b| b.1.cmp(a.1)); // Sorting in descending order

    for (command, count) in sorted_pairs {
        let mut span: Vec<Span> = Vec::new();
        let count_text = format!(" ({})", count);
        span.push(Span::styled(command, Style::default().fg(Color::Rgb(120,120,120))));
        span.push(Span::styled(count_text, Style::default().fg(Color::Rgb(120,120,120))));
        spans.push(Spans::from(span));
    }

    Paragraph::new(spans)
        .block(block)
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