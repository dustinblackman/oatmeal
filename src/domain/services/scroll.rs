use ratatui::widgets::ScrollbarState;

#[derive(Default)]
pub struct Scroll {
    list_length: u16,
    viewport_length: u16,
    pub position: u16,
    pub scrollbar_state: ScrollbarState,
}

impl Scroll {
    pub fn up(&mut self) {
        self.position = self.position.saturating_sub(1);
        self.scrollbar_state.prev();
    }

    pub fn up_page(&mut self) {
        for _ in 0..10 {
            self.up();
        }
    }

    pub fn down(&mut self) {
        let mut clamp: u16 = 0;
        if self.list_length > self.viewport_length {
            clamp = self.list_length - self.viewport_length + 1;
        }

        self.position = self
            .position
            .saturating_add(1)
            .clamp(0, clamp.saturating_sub(1));
        self.scrollbar_state.next();
    }

    pub fn down_page(&mut self) {
        for _ in 0..10 {
            self.down();
        }
    }

    pub fn last(&mut self) {
        self.position = 0;
        if self.list_length > self.viewport_length {
            self.position = self.list_length - self.viewport_length;
        }

        self.scrollbar_state.last();
    }

    pub fn set_state(&mut self, list_length: u16, viewport_length: u16) {
        self.list_length = list_length;
        self.viewport_length = viewport_length;
        self.scrollbar_state = self
            .scrollbar_state
            .content_length(list_length)
            .viewport_content_length(viewport_length);
    }
}
