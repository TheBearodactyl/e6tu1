use {
    crate::models::E6Post,
    ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget},
    },
    ratatui_image::{StatefulImage, protocol::StatefulProtocol},
};

pub struct PostViewer<'a> {
    post: &'a E6Post,
}

impl<'a> PostViewer<'a> {
    pub fn new(post: &'a E6Post) -> Self {
        Self { post }
    }
}

impl<'a> StatefulWidget for PostViewer<'a> {
    type State = Option<StatefulProtocol>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!("Post #{} - Full View", self.post.id))
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        block.render(area, buf);

        if let Some(protocol) = state {
            let image_widget = StatefulImage::default();
            StatefulWidget::render(image_widget, inner, buf, protocol);
        } else {
            let placeholder = Paragraph::new("Loading image...")
                .style(Style::default().fg(Color::DarkGray))
                .centered();
            placeholder.render(inner, buf);
        }
    }
}
