use ratatui::prelude::Alignment;
use ratatui::prelude::Backend;
use ratatui::prelude::Rect;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

#[derive(Default)]
pub struct Loading {}

impl Loading {
    pub fn render<B: Backend>(&self, frame: &mut Frame<B>, rect: Rect) {
        frame.render_widget(
            Paragraph::new("Loading...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .padding(Padding::new(1, 1, 0, 0)),
                )
                .alignment(Alignment::Center),
            rect,
        );
    }
}
