use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;

pub struct TextArea {}

impl<'a> TextArea {
    pub fn default() -> tui_textarea::TextArea<'a> {
        let mut textarea = tui_textarea::TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title("Enter prompt")
                .padding(Padding::new(1, 1, 0, 0)),
        );

        return textarea;
    }
}
