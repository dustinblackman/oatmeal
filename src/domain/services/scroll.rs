use ratatui::widgets::ScrollbarState;

#[derive(Default)]
pub struct Scroll {
    list_length: usize,
    viewport_length: usize,
    pub position: usize,
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
        let mut clamp: usize = 0;
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

    pub fn set_state(&mut self, list_length: usize, viewport_length: usize) {
        self.list_length = list_length;
        self.viewport_length = viewport_length;

        let mut content_length = list_length.saturating_sub(viewport_length);
        if content_length == 0 {
            content_length = 1;
        }

        self.scrollbar_state = self
            .scrollbar_state
            .content_length(content_length)
            .viewport_content_length(viewport_length.saturating_sub(2));
    }
}
