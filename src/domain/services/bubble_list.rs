use std::collections::HashMap;

use ratatui::prelude::Rect;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use syntect::highlighting::Theme;

use super::Bubble;
use super::BubbleAlignment;
use crate::domain::models::Author;
use crate::domain::models::Message;

#[cfg(test)]
#[path = "bubble_list_test.rs"]
mod tests;

struct BubbleCacheEntry<'a> {
    codeblocks_count: usize,
    text_len: usize,
    lines: Vec<Line<'a>>,
}

pub struct BubbleList<'a> {
    cache: HashMap<usize, BubbleCacheEntry<'a>>,
    line_width: usize,
    lines_len: usize,
    theme: Theme,
}

impl<'a> BubbleList<'a> {
    pub fn new(theme: Theme) -> BubbleList<'a> {
        return BubbleList {
            cache: HashMap::new(),
            line_width: 0,
            lines_len: 0,
            theme,
        };
    }

    pub fn set_messages(&mut self, messages: &[Message], line_width: usize) {
        if self.line_width != line_width {
            self.cache.clear();
            self.line_width = line_width;
        }

        let mut total_codeblock_counter = 0;
        self.lines_len = messages
            .iter()
            .enumerate()
            .map(|(idx, message)| {
                if self.cache.contains_key(&idx) {
                    let cache_entry = self.cache.get(&idx).unwrap();
                    if idx < (messages.len() - 1) || message.text.len() == cache_entry.text_len {
                        total_codeblock_counter += cache_entry.codeblocks_count;
                        return cache_entry.lines.len();
                    }
                }

                let mut align = BubbleAlignment::Left;
                if message.author == Author::User {
                    align = BubbleAlignment::Right;
                }

                let bubble_lines = Bubble::new(message, align, line_width, total_codeblock_counter)
                    .as_lines(&self.theme);
                let bubble_line_len = bubble_lines.len();

                let codeblocks_count = message.codeblocks().len();
                total_codeblock_counter += codeblocks_count;

                self.cache.insert(
                    idx,
                    BubbleCacheEntry {
                        codeblocks_count,
                        text_len: message.text.len(),
                        lines: bubble_lines,
                    },
                );

                return bubble_line_len;
            })
            .sum();
    }

    pub fn len(&self) -> usize {
        return self.lines_len;
    }

    pub fn render(&self, frame: &mut Frame, rect: Rect, scroll: usize) {
        let mut indexes: Vec<usize> = self.cache.keys().cloned().collect();
        indexes.sort();
        let lines: Vec<Line<'a>> = indexes
            .iter()
            .flat_map(|idx| {
                return self.cache.get(idx).unwrap().lines.to_owned();
            })
            .collect();

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default())
                .scroll((scroll.try_into().unwrap(), 0)),
            rect,
        );
    }
}
