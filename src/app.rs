use {
    crate::{
        api::E621Client, event::AppEvent, models::E6Post, widgets::post_popup::E6PostPopupState,
    },
    color_eyre::eyre::Result,
    crossterm::event::KeyCode,
    ratatui::widgets::ListState,
    ratatui_image::picker::Picker,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Input,
    Loading,
    SearchResults,
    Viewing,
    FullImageView,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    TagSearch,
    PostId,
}

#[derive(Clone, Debug)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub message: String,
}

impl DownloadProgress {
    pub fn new(message: String) -> Self {
        Self {
            total_bytes: 0,
            downloaded_bytes: 0,
            message,
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.downloaded_bytes as f64 / self.total_bytes as f64
        }
    }
}

pub struct App {
    pub state: AppState,
    pub input_mode: InputMode,
    pub tag_input: String,
    pub tag_cursor_position: usize,
    pub id_input: String,
    pub id_cursor_position: usize,
    pub post: Option<E6Post>,
    pub search_results: Vec<E6Post>,
    pub list_state: ListState,
    pub popup_state: E6PostPopupState,
    pub picker: Picker,
    pub error_message: Option<String>,
    pub download_progress: Option<DownloadProgress>,

    // Pending operations
    pending_search: bool,
    pending_fetch: bool,
    pending_load_image: bool,
    pending_download: bool,
    pending_open_browser: bool,

    client: E621Client,
}

impl App {
    pub fn new() -> Self {
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
            download_progress: None,
            pending_search: false,
            pending_fetch: false,
            pending_load_image: false,
            pending_download: false,
            pending_open_browser: false,
            client: E621Client::new(),
        }
    }

    pub async fn handle_event(&mut self, event: AppEvent) -> Result<bool> {
        if let AppEvent::Key(key) = event {
            match self.state {
                AppState::Input => self.handle_input_key(key),
                AppState::Loading => {}
                AppState::SearchResults => self.handle_search_results_key(key),
                AppState::Viewing => self.handle_viewing_key(key),
                AppState::FullImageView => self.handle_full_image_key(key),
                AppState::Error => {
                    self.state = AppState::Input;
                    self.error_message = None;
                }
            }
        }
        Ok(true)
    }

    pub async fn process_pending_operations(&mut self) -> Result<()> {
        if self.pending_search {
            self.pending_search = false;
            self.download_progress = Some(DownloadProgress::new("Searching posts...".to_string()));
            if let Err(e) = self.search_posts().await {
                self.error_message = Some(format!("Failed to search posts: {}", e));
                self.state = AppState::Error;
            }
            self.download_progress = None;
        }

        if self.pending_fetch {
            self.pending_fetch = false;
            self.download_progress = Some(DownloadProgress::new("Fetching post...".to_string()));
            match self.fetch_post().await {
                Ok(_) => self.pending_load_image = true,
                Err(e) => {
                    self.error_message = Some(format!("Failed to fetch post: {}", e));
                    self.state = AppState::Error;
                }
            }
            self.download_progress = None;
        }

        if self.pending_load_image {
            self.pending_load_image = false;
            self.download_progress = Some(DownloadProgress::new("Loading image...".to_string()));
            if let Err(e) = self.load_image().await {
                eprintln!("Failed to load image: {}", e);
            }
            self.download_progress = None;
        }

        if self.pending_download {
            self.pending_download = false;
            if let Err(e) = self.download_post().await {
                self.error_message = Some(format!("Failed to download: {}", e));
                self.state = AppState::Error;
            }
        }

        if self.pending_open_browser {
            self.pending_open_browser = false;
            if let Err(e) = self.open_in_browser() {
                self.error_message = Some(format!("Failed to open browser: {}", e));
                self.state = AppState::Error;
            }
        }

        Ok(())
    }

    fn handle_input_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('q') => std::process::exit(0),
            KeyCode::Enter => match self.input_mode {
                InputMode::TagSearch => {
                    if !self.tag_input.is_empty() {
                        self.state = AppState::Loading;
                        self.pending_search = true;
                    }
                }
                InputMode::PostId => {
                    if !self.id_input.is_empty() {
                        self.state = AppState::Loading;
                        self.pending_fetch = true;
                    }
                }
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
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected()
                    && let Some(post) = self.search_results.get(selected).cloned()
                {
                    self.post = Some(post);
                    self.state = AppState::Viewing;
                    self.popup_state = E6PostPopupState::new();
                    self.pending_load_image = true;
                }
            }
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
            KeyCode::Up => {
                self.popup_state.scroll_offset = self.popup_state.scroll_offset.saturating_sub(1);
            }
            KeyCode::Down => {
                self.popup_state.scroll_offset = self.popup_state.scroll_offset.saturating_add(1);
            }
            KeyCode::Char('d') => {
                self.pending_download = true;
            }
            KeyCode::Char('o') => {
                self.pending_open_browser = true;
            }
            KeyCode::Char('f') => {
                self.state = AppState::FullImageView;
            }
            _ => {}
        }
    }

    fn handle_full_image_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('f') => {
                self.state = AppState::Viewing;
            }
            _ => {}
        }
    }

    // Input handling methods
    pub fn active_input(&self) -> &str {
        match self.input_mode {
            InputMode::TagSearch => &self.tag_input,
            InputMode::PostId => &self.id_input,
        }
    }

    pub fn active_cursor(&self) -> usize {
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

    fn switch_input_mode(&mut self) {
        self.input_mode = match self.input_mode {
            InputMode::TagSearch => InputMode::PostId,
            InputMode::PostId => InputMode::TagSearch,
        };
    }

    // API operations
    async fn search_posts(&mut self) -> Result<()> {
        let posts = self.client.search_posts(&self.tag_input).await?;

        if posts.is_empty() {
            self.error_message = Some("No posts found for this search".to_string());
            self.state = AppState::Error;
        } else {
            self.search_results = posts;
            self.state = AppState::SearchResults;
            self.list_state.select(Some(0));
        }

        Ok(())
    }

    async fn fetch_post(&mut self) -> Result<()> {
        let post = self.client.fetch_post(&self.id_input).await?;
        self.post = Some(post);
        self.state = AppState::Viewing;
        self.popup_state = E6PostPopupState::new();
        Ok(())
    }

    async fn load_image(&mut self) -> Result<()> {
        if let Some(ref post) = self.post
            && let Some(ref url) = post.file.url
        {
            let img = self.client.download_image(url).await?;
            self.popup_state.image_protocol = Some(self.picker.new_resize_protocol(img));
        }
        Ok(())
    }

    async fn download_post(&mut self) -> Result<()> {
        if let Some(ref post) = self.post {
            self.client
                .download_post_to_file(post, &mut self.download_progress)
                .await?;
        }
        Ok(())
    }

    fn open_in_browser(&self) -> Result<()> {
        if let Some(ref post) = self.post {
            let url = format!("https://e621.net/posts/{}", post.id);
            open::that(url)?;
        }
        Ok(())
    }
}
