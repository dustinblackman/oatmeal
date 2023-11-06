use std::collections::HashMap;

use ratatui::prelude::Backend;
use ratatui::prelude::Rect;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use syntect::highlighting::Theme;

use crate::domain::models::Author;
use crate::domain::models::Bubble;
use crate::domain::models::BubbleAlignment;
use crate::domain::models::Message;

#[cfg(test)]
#[path = "bubble_list_test.rs"]
mod tests;

struct BubbleCacheEntry<'a> {
    codeblocks_count: usize,
    text: String,
    lines: Vec<Line<'a>>,
}

pub struct BubbleList<'a> {
    cache: HashMap<usize, BubbleCacheEntry<'a>>,
    line_width: u16,
    lines: Vec<Line<'a>>,
    theme: Theme,
}

impl<'a> BubbleList<'a> {
    pub fn new(theme: Theme) -> BubbleList<'a> {
        return BubbleList {
            cache: HashMap::new(),
            line_width: 0,
            lines: vec![],
            theme,
        };
    }

    pub fn set_messages(&mut self, messages: &[Message], line_width: u16) {
        if self.line_width != line_width {
            self.cache.clear();
            self.line_width = line_width;
        }

        let mut total_codeblock_counter = 0;
        self.lines = messages
            .iter()
            .filter(|message| {
                return !message.text.is_empty();
            })
            .enumerate()
            .flat_map(|(idx, message)| {
                if self.cache.contains_key(&idx) {
                    let cache_entry = self.cache.get(&idx).unwrap();
                    if message.text == cache_entry.text {
                        total_codeblock_counter += cache_entry.codeblocks_count;
                        return cache_entry.lines.to_owned();
                    }
                }

                let mut align = BubbleAlignment::Left;
                if message.author == Author::User {
                    align = BubbleAlignment::Right;
                }

                let bubble_lines = Bubble::new(message.clone()).as_lines(
                    align,
                    &self.theme,
                    line_width,
                    total_codeblock_counter,
                );

                let codeblocks_count = message.codeblocks().len();
                total_codeblock_counter += codeblocks_count;

                self.cache.insert(
                    idx,
                    BubbleCacheEntry {
                        codeblocks_count,
                        text: message.text.to_string(),
                        lines: bubble_lines.to_owned(),
                    },
                );

                return bubble_lines;
            })
            .collect::<Vec<Line>>();
    }

    pub fn len(&self) -> usize {
        return self.lines.len();
    }

    pub fn render<B: Backend>(&mut self, frame: &mut Frame<B>, rect: Rect, scroll: u16) {
        frame.render_widget(
            Paragraph::new(self.lines.to_owned())
                .block(Block::default())
                .scroll((scroll, 0)),
            rect,
        );
    }
}
