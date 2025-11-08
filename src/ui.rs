use {
    crate::{
        app::{App, AppState, InputMode},
        widgets::{post_popup::E6PostPopup, post_viewer::PostViewer},
    },
    ratatui::{
        Frame,
        layout::{Constraint, Layout, Rect},
        style::{Color, Modifier, Style},
        text::Span,
        widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    },
};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(f.area());

    render_title(f, chunks[0]);
    render_tag_input(f, app, chunks[1]);

    match app.state {
        AppState::Input => render_input_screen(f, app, chunks[2]),
        AppState::Loading => render_loading(f, app, chunks[2]),
        AppState::SearchResults => render_search_results(f, app, chunks[2]),
        AppState::Viewing => render_post_view(f, app),
        AppState::FullImageView => render_full_image(f, app),
        AppState::Error => render_error(f, app, chunks[2]),
    }

    render_help(f, app, chunks[3]);

    if let Some(ref progress) = app.download_progress {
        render_progress_overlay(f, progress, f.area());
    }
}

fn render_progress_overlay(f: &mut Frame, progress: &crate::app::DownloadProgress, area: Rect) {
    let progress_area = centered_rect(60, 10, area);
    f.render_widget(Clear, progress_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Download Progress")
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(progress_area);
    f.render_widget(block, progress_area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(inner_area);

    let message =
        Paragraph::new(progress.message.as_str()).style(Style::default().fg(Color::White));
    f.render_widget(message, chunks[0]);

    let percentage = (progress.ratio() * 100.0) as u16;
    let label = format!(
        "{:.2} MB / {:.2} MB ({}%)",
        progress.downloaded_bytes as f64 / 1_048_576.0,
        progress.total_bytes as f64 / 1_048_576.0,
        percentage
    );

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent(percentage)
        .label(label);
    f.render_widget(gauge, chunks[1]);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("E6TU1")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_tag_input(f: &mut Frame, app: &App, area: Rect) {
    let tag_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: 3,
    };

    let tag_input_style = if app.input_mode == InputMode::TagSearch {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let tag_input = Paragraph::new(app.tag_input.as_str())
        .style(tag_input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Search by Tags (Tab to switch)")
                .border_style(if app.input_mode == InputMode::TagSearch {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );
    f.render_widget(tag_input, tag_area);

    if app.state == AppState::Input && app.input_mode == InputMode::TagSearch {
        f.set_cursor_position((
            tag_area.x + app.tag_cursor_position as u16 + 1,
            tag_area.y + 1,
        ));
    }
}

fn render_input_screen(f: &mut Frame, app: &mut App, area: Rect) {
    let id_area = Rect {
        x: area.x + 2,
        y: area.y + 5,
        width: area.width.saturating_sub(4),
        height: 3,
    };

    let id_input_style = if app.input_mode == InputMode::PostId {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let id_input = Paragraph::new(app.id_input.as_str())
        .style(id_input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Or Enter Post ID")
                .border_style(if app.input_mode == InputMode::PostId {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        );
    f.render_widget(id_input, id_area);

    if app.input_mode == InputMode::PostId {
        f.set_cursor_position((id_area.x + app.id_cursor_position as u16 + 1, id_area.y + 1));
    }
}

fn render_loading(f: &mut Frame, app: &App, area: Rect) {
    let message = if let Some(ref progress) = app.download_progress {
        progress.message.as_str()
    } else {
        "Loading..."
    };

    let loading = Paragraph::new(message)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL))
        .centered();
    f.render_widget(loading, area);
}

fn render_search_results(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|post| {
            let rating_color = match post.rating.as_str() {
                "s" => Color::Green,
                "q" => Color::Yellow,
                "e" => Color::Red,
                _ => Color::White,
            };

            let content = ratatui::text::Line::from(vec![
                Span::styled(format!("#{:<8}", post.id), Style::default().fg(Color::Cyan)),
                Span::raw(" | "),
                Span::styled(
                    format!("{:<2}", post.rating.to_uppercase()),
                    Style::default()
                        .fg(rating_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" | Score: "),
                Span::styled(
                    format!("{:<5}", post.score.total),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(" | "),
                Span::raw(
                    post.tags
                        .artist
                        .first()
                        .map(|s| s.as_str())
                        .unwrap_or("unknown"),
                ),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Search Results ({} posts)",
            app.search_results.len()
        )))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_post_view(f: &mut Frame, app: &mut App) {
    if let Some(ref post) = app.post {
        let popup = E6PostPopup::new(post);
        f.render_stateful_widget(popup, f.area(), &mut app.popup_state);
    }
}

fn render_full_image(f: &mut Frame, app: &mut App) {
    if let Some(ref post) = app.post {
        let viewer = PostViewer::new(post);
        f.render_stateful_widget(viewer, f.area(), &mut app.popup_state.image_protocol);
    }
}

fn render_error(f: &mut Frame, app: &App, area: Rect) {
    if let Some(ref error) = app.error_message {
        let error_area = centered_rect(60, 20, area);
        f.render_widget(Clear, error_area);

        let error_text = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .border_style(Style::default().fg(Color::Red)),
            )
            .wrap(Wrap { trim: true })
            .centered();
        f.render_widget(error_text, error_area);
    }
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.state {
        AppState::Input => "Enter: Submit | Tab: Switch Input | Esc: Clear | q: Quit",
        AppState::Loading => "Loading...",
        AppState::SearchResults => "↑↓: Navigate | Enter: View Post | q/Esc: Back",
        AppState::Viewing => {
            if app.popup_state.image_protocol.is_some() {
                "↑↓: Scroll | d: Download | o: Open Browser | f: Full Image | q/Esc: Back"
            } else {
                "Loading image... | q/Esc: Back"
            }
        }
        AppState::FullImageView => "f/q/Esc: Exit Full Screen",
        AppState::Error => "Press any key to continue",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .centered();
    f.render_widget(help, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
