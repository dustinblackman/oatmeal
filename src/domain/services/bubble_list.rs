use std::collections::HashMap;

use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::text::Line;
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

    pub fn render(&self, rect: Rect, buf: &mut Buffer, scroll_index: u16) {
        let mut cache_keys: Vec<usize> = self.cache.keys().cloned().collect();
        cache_keys.sort();

        let mut line_idx = 0;
        let mut should_break = false;

        for cache_key in cache_keys {
            for line in self.cache.get(&cache_key).unwrap().lines.as_slice() {
                if line_idx < scroll_index {
                    line_idx += 1;
                    continue;
                }

                if (line_idx - scroll_index) >= rect.height {
                    should_break = true;
                    break;
                }

                buf.set_line(0, line_idx - scroll_index, line, rect.width);
                line_idx += 1;
            }

            if should_break {
                break;
            }
        }
    }
}
