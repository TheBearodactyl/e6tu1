use {
    color_eyre::eyre::{self, Result},
    crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    futures::StreamExt,
    models::{E6Post, E6PostResponse, E6PostsResponse},
    ratatui::{
        Frame, Terminal,
        backend::CrosstermBackend,
        layout::{Constraint, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    },
    ratatui_image::picker::Picker,
    std::{
        fs::File,
        io::{self, Stdout, Write},
    },
    widget::{E6PostPopup, E6PostPopupState},
};

mod models;
mod widget;

const USER_AGENT: &str = "E6TU1/1.0 (by bearodactyl on e621)";
const BASE_URL: &str = "https://e621.net";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    Input,
    Loading,
    SearchResults,
    Viewing,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    TagSearch,
    PostId,
}

struct App {
    state: AppState,
    input_mode: InputMode,
    tag_input: String,
    tag_cursor_position: usize,
    id_input: String,
    id_cursor_position: usize,
    post: Option<E6Post>,
    search_results: Vec<E6Post>,
    list_state: ListState,
    popup_state: E6PostPopupState,
    picker: Picker,
    error_message: Option<String>,
    image_loaded: bool,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::Input,
            input_mode: InputMode::TagSearch,
            tag_input: String::new(),
            tag_cursor_position: 0,
            id_input: String::new(),
            id_cursor_position: 0,
            post: None,
            search_results: Vec::new(),
            list_state: ListState::default(),
            popup_state: E6PostPopupState::new(),
            picker: Picker::from_query_stdio().unwrap(),
            error_message: None,
            image_loaded: false,
        }
    }

    fn active_input(&self) -> &str {
        match self.input_mode {
            InputMode::TagSearch => &self.tag_input,
            InputMode::PostId => &self.id_input,
        }
    }

    fn active_cursor(&self) -> usize {
        match self.input_mode {
            InputMode::TagSearch => self.tag_cursor_position,
            InputMode::PostId => self.id_cursor_position,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.active_cursor().saturating_sub(1);
        self.set_cursor(self.clamp_cursor(cursor_moved_left));
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.active_cursor().saturating_add(1);
        self.set_cursor(self.clamp_cursor(cursor_moved_right));
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        match self.input_mode {
            InputMode::TagSearch => {
                self.tag_input.insert(index, new_char);
            }
            InputMode::PostId => {
                if new_char.is_ascii_digit() {
                    self.id_input.insert(index, new_char);
                }
            }
        }
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.active_input()
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.active_cursor())
            .unwrap_or(self.active_input().len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.active_cursor() != 0;
        if is_not_cursor_leftmost {
            let current_index = self.active_cursor();
            let from_left_to_current_index = current_index - 1;

            let (before_char_to_delete, after_char_to_delete) = match self.input_mode {
                InputMode::TagSearch => (
                    self.tag_input.chars().take(from_left_to_current_index),
                    self.tag_input.chars().skip(current_index),
                ),
                InputMode::PostId => (
                    self.id_input.chars().take(from_left_to_current_index),
                    self.id_input.chars().skip(current_index),
                ),
            };

            let new_input: String = before_char_to_delete.chain(after_char_to_delete).collect();

            match self.input_mode {
                InputMode::TagSearch => self.tag_input = new_input,
                InputMode::PostId => self.id_input = new_input,
            }

            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.active_input().chars().count())
    }

    fn set_cursor(&mut self, pos: usize) {
        match self.input_mode {
            InputMode::TagSearch => self.tag_cursor_position = pos,
            InputMode::PostId => self.id_cursor_position = pos,
        }
    }

    fn _reset_cursor(&mut self) {
        match self.input_mode {
            InputMode::TagSearch => self.tag_cursor_position = 0,
            InputMode::PostId => self.id_cursor_position = 0,
        }
    }

    fn switch_input_mode(&mut self) {
        self.input_mode = match self.input_mode {
            InputMode::TagSearch => InputMode::PostId,
            InputMode::PostId => InputMode::TagSearch,
        };
    }

    fn submit_search(&mut self) {
        if !self.tag_input.is_empty() {
            self.state = AppState::Loading;
        }
    }

    fn submit_post_id(&mut self) {
        if !self.id_input.is_empty() {
            self.state = AppState::Loading;
        }
    }

    async fn search_posts(&mut self) -> Result<()> {
        let tags = self.tag_input.clone();
        let url = format!(
            "{}/posts.json?tags={}&limit=50",
            BASE_URL,
            urlencoding::encode(&tags)
        );

        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(eyre::Error::msg(format!(
                "Failed to search posts: HTTP {}",
                response.status()
            )));
        }

        let posts_response: E6PostsResponse = response.json().await?;
        self.search_results = posts_response.posts;

        if self.search_results.is_empty() {
            self.error_message = Some("No posts found for this search".to_string());
            self.state = AppState::Error;
        } else {
            self.state = AppState::SearchResults;
            self.list_state.select(Some(0));
        }

        Ok(())
    }

    async fn fetch_post(&mut self) -> Result<()> {
        let post_id = self.id_input.clone();
        let url = format!("{}/posts/{}.json", BASE_URL, post_id);

        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(eyre::Error::msg(format!(
                "Failed to fetch post: HTTP {}",
                response.status()
            )));
        }

        let post_response: E6PostResponse = response.json().await?;
        self.post = Some(post_response.post);
        self.state = AppState::Viewing;
        self.image_loaded = false;
        self.popup_state = E6PostPopupState::new();

        Ok(())
    }

    async fn download_post(&mut self) -> Result<()> {
        let post_id = self.id_input.clone();
        let url = format!("{}/posts/{}.json", BASE_URL, post_id);
        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
        let response = client.get(&url).send().await?;
        let post_response: E6PostResponse = response.json().await?;
        let image_url = post_response.post.file.url.unwrap();
        let resp = client.get(image_url).send().await?;

        if !resp.status().is_success() {
            eyre::bail!("Download failed with status: {}", resp.status());
        }

        let mut file = File::create(format!(
            "{}.{}",
            post_response.post.id, post_response.post.file.ext
        ))?;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;

            file.write_all(&chunk)?;
        }

        file.flush()?;

        Ok(())
    }

    async fn load_image(&mut self) -> Result<()> {
        if let Some(ref post) = self.post
            && let Some(ref url) = post.file.url
        {
            let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

            let response = client.get(url).send().await?;
            let bytes = response.bytes().await?;

            let img = image::load_from_memory(&bytes)?;
            self.popup_state
                .set_image(&mut self.picker, img)
                .expect("Failed to set image");
            self.image_loaded = true;
        }

        Ok(())
    }

    fn select_from_search(&mut self) {
        if let Some(selected) = self.list_state.selected()
            && let Some(post) = self.search_results.get(selected).cloned()
        {
            self.post = Some(post);
            self.state = AppState::Viewing;
            self.image_loaded = false;
            self.popup_state = E6PostPopupState::new();
        }
    }

    fn handle_input_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Enter => match self.input_mode {
                InputMode::TagSearch => self.submit_search(),
                InputMode::PostId => self.submit_post_id(),
            },
            KeyCode::Char(c) => self.enter_char(c),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Tab => self.switch_input_mode(),
            KeyCode::Esc => match self.input_mode {
                InputMode::TagSearch => {
                    self.tag_input.clear();
                    self.tag_cursor_position = 0;
                }
                InputMode::PostId => {
                    self.id_input.clear();
                    self.id_cursor_position = 0;
                }
            },
            _ => {}
        }
    }

    fn handle_search_results_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = AppState::Input;
                self.search_results.clear();
                self.list_state.select(None);
            }
            KeyCode::Up => {
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i > 0 {
                            i - 1
                        } else {
                            self.search_results.len().saturating_sub(1)
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Down => {
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= self.search_results.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Enter => self.select_from_search(),
            _ => {}
        }
    }

    fn handle_viewing_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.search_results.is_empty() {
                    self.state = AppState::Input;
                    self.post = None;
                    self.popup_state = E6PostPopupState::new();
                } else {
                    self.state = AppState::SearchResults;
                    self.post = None;
                    self.popup_state = E6PostPopupState::new();
                }
            }
            KeyCode::Up => self.popup_state.scroll_up(),
            KeyCode::Down => self.popup_state.scroll_down(),
            _ => {}
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(f.area());

    let title = Paragraph::new("E6TU1")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

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
    f.render_widget(tag_input, chunks[1]);

    if app.state == AppState::Input && app.input_mode == InputMode::TagSearch {
        f.set_cursor_position((
            chunks[1].x + app.tag_cursor_position as u16 + 1,
            chunks[1].y + 1,
        ));
    }

    match app.state {
        AppState::Input => {
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
            f.render_widget(id_input, chunks[2]);

            if app.input_mode == InputMode::PostId {
                f.set_cursor_position((
                    chunks[2].x + app.id_cursor_position as u16 + 1,
                    chunks[2].y + 1,
                ));
            }
        }
        AppState::Loading => {
            let loading = Paragraph::new("Loading...")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL))
                .centered();
            f.render_widget(loading, chunks[2]);
        }
        AppState::SearchResults => {
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

                    let content = Line::from(vec![
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

            f.render_stateful_widget(list, chunks[2], &mut app.list_state);
        }
        AppState::Viewing => {
            if let Some(ref post) = app.post {
                let popup = E6PostPopup::new(post);
                f.render_stateful_widget(popup, f.area(), &mut app.popup_state);
            }
        }
        AppState::Error => {
            if let Some(ref error) = app.error_message {
                let error_area = centered_rect(60, 20, chunks[2]);
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
    }

    let help_text = match app.state {
        AppState::Input => "Enter: Submit | Tab: Switch Input | Esc: Clear | q: Quit",
        AppState::Loading => "Loading...",
        AppState::SearchResults => "↑↓: Navigate | Enter: View Post | q/Esc: Back",
        AppState::Viewing => {
            if app.image_loaded {
                "↑↓: Scroll Info | q/Esc: Back | d: Download"
            } else {
                "Loading image... | q/Esc: Back"
            }
        }
        AppState::Error => "Press any key to continue",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .centered();
    f.render_widget(help, chunks[3]);
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

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let mut should_search = false;
    let mut should_fetch = false;
    let mut should_load_image = false;

    loop {
        terminal.draw(|f| {
            ui(f, app);
        })?;

        if should_search {
            should_search = false;
            match app.search_posts().await {
                Ok(_) => {}
                Err(e) => {
                    app.error_message = Some(format!("Failed to search posts: {}", e));
                    app.state = AppState::Error;
                }
            }
        }

        if should_fetch {
            should_fetch = false;
            match app.fetch_post().await {
                Ok(_) => {
                    should_load_image = true;
                }
                Err(e) => {
                    app.error_message = Some(format!("Failed to fetch post: {}", e));
                    app.state = AppState::Error;
                }
            }
        }

        if should_load_image {
            should_load_image = false;
            if let Err(e) = app.load_image().await {
                eprintln!("Failed to load image: {}", e);
            }
        }

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.state {
                AppState::Input => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Enter => match app.input_mode {
                        InputMode::TagSearch => {
                            if !app.tag_input.is_empty() {
                                should_search = true;
                            }
                        }
                        InputMode::PostId => {
                            if !app.id_input.is_empty() {
                                should_fetch = true;
                            }
                        }
                    },
                    _ => app.handle_input_key(key.code),
                },
                AppState::Loading => {}
                AppState::SearchResults => {
                    app.handle_search_results_key(key.code);
                    if app.state == AppState::Viewing {
                        should_load_image = true;
                    }
                }
                AppState::Viewing => {
                    app.handle_viewing_key(key.code);
                }
                AppState::Error => {
                    app.state = AppState::Input;
                    app.error_message = None;
                }
            }
        }
    }
}
