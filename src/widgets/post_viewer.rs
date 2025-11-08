use {
    crate::{anim::ImageProtocol, models::E6Post},
    ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget},
    },
    ratatui_image::{Resize, StatefulImage},
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
    type State = Option<ImageProtocol>;

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

        if let Some(image_protocol) = state {
            image_protocol.try_advance();

            let protocol = image_protocol.current_protocol_mut();
            let image_widget = StatefulImage::new().resize(Resize::Fit(None));
            StatefulWidget::render(image_widget, inner, buf, protocol);
        } else {
            let placeholder = Paragraph::new("Loading image...")
                .style(Style::default().fg(Color::DarkGray))
                .centered();
            placeholder.render(inner, buf);
        }
    }
}
