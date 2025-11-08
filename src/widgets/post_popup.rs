use {
    crate::models::E6Post,
    ratatui::{
        buffer::Buffer,
        layout::{Constraint, Flex, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget, Wrap},
    },
    ratatui_image::{StatefulImage, protocol::StatefulProtocol},
};

pub struct E6PostPopupState {
    pub image_protocol: Option<StatefulProtocol>,
    pub scroll_offset: u16,
}

impl E6PostPopupState {
    pub fn new() -> Self {
        Self {
            image_protocol: None,
            scroll_offset: 0,
        }
    }
}

pub struct E6PostPopup<'a> {
    post: &'a E6Post,
    title: String,
}

impl<'a> E6PostPopup<'a> {
    pub fn new(post: &'a E6Post) -> Self {
        Self {
            post,
            title: format!("Post #{}", post.id),
        }
    }

    fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn build_info_text(&self) -> Text<'a> {
        let mut lines = Vec::new();

        lines.push(Line::from(vec![
            Span::styled(
                "ID: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(self.post.id.to_string()),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Rating: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&self.post.rating),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Score: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "↑{} ↓{} ({})",
                self.post.score.up, self.post.score.down, self.post.score.total
            )),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Favorites: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(self.post.fav_count.to_string()),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Resolution: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{}x{}",
                self.post.file.width, self.post.file.height
            )),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Format: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&self.post.file.ext),
        ]));

        lines.push(Line::from(vec![
            Span::styled(
                "Size: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{:.2} MB",
                self.post.file.size as f64 / 1_048_576.0
            )),
        ]));

        if !self.post.tags.artist.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Artists:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::raw(self.post.tags.artist.join(", "))));
        }

        if !self.post.tags.character.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Characters:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::raw(self.post.tags.character.join(", "))));
        }

        if !self.post.tags.species.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Species:",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::raw(self.post.tags.species.join(", "))));
        }

        if !self.post.description.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::raw(""));

            for desc_line in self.post.description.lines() {
                lines.push(Line::raw(desc_line));
            }
        }

        if !self.post.sources.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Sources:",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )));
            for source in &self.post.sources {
                lines.push(Line::from(Span::styled(
                    source,
                    Style::default().fg(Color::Blue),
                )));
            }
        }

        Text::from(lines)
    }
}

impl<'a> StatefulWidget for E6PostPopup<'a> {
    type State = E6PostPopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);

        let popup_area = Self::popup_area(area, 80, 85);

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(self.title.clone())
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        outer_block.clone().render(popup_area, buf);

        let inner_area = outer_block.inner(popup_area);

        let horizontal =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let [image_area, info_area] = horizontal.areas(inner_area);

        if let Some(ref mut protocol) = state.image_protocol {
            let image_block = Block::default()
                .borders(Borders::ALL)
                .title("Preview")
                .border_style(Style::default().fg(Color::Cyan));

            let image_inner = image_block.inner(image_area);
            image_block.render(image_area, buf);

            let image_widget = StatefulImage::default();
            StatefulWidget::render(image_widget, image_inner, buf, protocol);
        } else {
            let placeholder_block = Block::default()
                .borders(Borders::ALL)
                .title("Preview")
                .border_style(Style::default().fg(Color::DarkGray));

            placeholder_block.clone().render(image_area, buf);

            let placeholder_inner = placeholder_block.inner(image_area);
            let placeholder_text = Paragraph::new("Loading image...")
                .style(Style::default().fg(Color::DarkGray))
                .centered()
                .wrap(Wrap { trim: true });

            placeholder_text.render(placeholder_inner, buf);
        }

        let info_block = Block::default()
            .borders(Borders::ALL)
            .title("Information")
            .border_style(Style::default().fg(Color::Green));

        let info_inner = info_block.inner(info_area);
        info_block.render(info_area, buf);

        let info_text = self.build_info_text();
        let info_paragraph = Paragraph::new(info_text.clone())
            .scroll((state.scroll_offset, 0))
            .wrap(Wrap { trim: true });

        info_paragraph.render(info_inner, buf);

        let max_scroll = info_text
            .lines
            .len()
            .saturating_sub(info_inner.height as usize);
        if max_scroll > 0 {
            let scroll_indicator = format!(" {}/{} ", state.scroll_offset, max_scroll);
            let indicator_len = scroll_indicator.len() as u16;
            let indicator_x = info_area.right().saturating_sub(indicator_len + 1);
            let indicator_y = info_area.bottom().saturating_sub(1);

            if indicator_x < info_area.right() && indicator_y < info_area.bottom() {
                buf.set_string(
                    indicator_x,
                    indicator_y,
                    scroll_indicator,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }
    }
}
