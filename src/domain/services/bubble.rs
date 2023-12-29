#[cfg(test)]
#[path = "bubble_test.rs"]
mod tests;

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;

use super::Syntaxes;
use super::SYNTAX_SET;
use crate::domain::models::Author;
use crate::domain::models::Message;
use crate::domain::models::MessageType;

#[derive(PartialEq, Eq)]
pub enum BubbleAlignment {
    Left,
    Right,
}

pub struct Bubble<'a> {
    alignment: BubbleAlignment,
    message: &'a Message,
    window_max_width: usize,
    codeblock_counter: usize,
}

pub struct BubbleConfig {
    pub bubble_padding: usize,
    pub border_elements_length: usize,
    pub outer_padding_percentage: f32,
}

fn repeat_from_subtractions(text: &str, subtractions: Vec<usize>) -> String {
    let count = subtractions
        .into_iter()
        .map(|e| {
            return i32::try_from(e).unwrap();
        })
        .reduce(|a, b| {
            return a - b;
        })
        .unwrap();

    if count <= 0 {
        return "".to_string();
    }

    return [text].repeat(count.try_into().unwrap()).join("");
}

impl<'a> Bubble<'_> {
    pub fn new(
        message: &'a Message,
        alignment: BubbleAlignment,
        window_max_width: usize,
        codeblock_counter: usize,
    ) -> Bubble {
        return Bubble {
            alignment,
            message,
            window_max_width,
            codeblock_counter,
        };
    }

    pub fn style_config() -> BubbleConfig {
        return BubbleConfig {
            // Unicode character border + padding.
            bubble_padding: 8,
            // left border + left padding + (text, not counted) + right padding + right border +
            // scrollbar.
            border_elements_length: 5,
            outer_padding_percentage: 0.04,
        };
    }

    pub fn as_lines(&mut self, theme: &Theme) -> Vec<Line<'a>> {
        // Lazy default
        let mut highlight = HighlightLines::new(Syntaxes::get("text"), theme);
        let mut in_codeblock = false;
        let mut lines: Vec<Line> = vec![];

        let max_line_length = self.get_max_line_length();

        for line in self.message.text.lines() {
            let mut spans = vec![];

            if line.trim().starts_with("```") {
                let lang = line.trim().replace("```", "");
                let syntax = Syntaxes::get(&lang);
                if !in_codeblock {
                    highlight = HighlightLines::new(syntax, theme);
                    in_codeblock = true;

                    self.codeblock_counter += 1;
                    spans = vec![
                        Span::from(line.to_owned()),
                        Span::styled(
                            format!(" ({})", self.codeblock_counter),
                            Style {
                                fg: Some(Color::White),
                                ..Style::default()
                            },
                        ),
                    ];
                } else {
                    in_codeblock = false;
                }
            } else if in_codeblock {
                // Highlighting doesn't work accurately unless each line is postfixed with '\n',
                // especially when dealing with multi-line code comments.
                let line_nl = format!("{line}\n");
                let highlighted = highlight.highlight_line(&line_nl, &SYNTAX_SET).unwrap();

                spans = highlighted
                    .iter()
                    .enumerate()
                    .map(|(idx, segment)| {
                        let (style, content) = segment;
                        let mut text = content.to_string();
                        if idx == highlighted.len() - 1 {
                            text = text.trim_end().to_string();
                        }

                        return Span::styled(
                            text,
                            Style {
                                fg: Syntaxes::translate_colour(style.foreground),
                                ..Style::default()
                            },
                        );
                    })
                    .collect();
            }

            if spans.is_empty() {
                spans = vec![Span::styled(line.to_owned(), Style::default())];
            }

            let mut split_spans = vec![];
            let mut line_char_count = 0;

            for span in spans {
                if span.content.len() + line_char_count <= max_line_length {
                    line_char_count += span.content.len();
                    split_spans.push(span);
                    continue;
                }

                let mut word_set: Vec<&str> = vec![];

                for word in span.content.split(' ') {
                    if word.len() + line_char_count > max_line_length {
                        split_spans.push(Span::styled(word_set.join(" "), span.style));
                        lines.push(self.spans_to_line(split_spans, max_line_length));

                        split_spans = vec![];
                        word_set = vec![];
                        line_char_count = 0;
                    }

                    word_set.push(word);
                    line_char_count += word.len() + 1;
                }

                split_spans.push(Span::styled(word_set.join(" "), span.style));
            }

            lines.push(self.spans_to_line(split_spans, max_line_length));
        }

        return self.wrap_lines_in_buddle(lines, max_line_length);
    }

    fn spans_to_line(&self, mut spans: Vec<Span<'a>>, max_line_length: usize) -> Line<'a> {
        let line_str_len: usize = spans.iter().map(|e| return e.content.len()).sum();
        let fill = repeat_from_subtractions(" ", vec![max_line_length, line_str_len]);
        let formatted_line_length =
            line_str_len + fill.len() + Bubble::style_config().bubble_padding;

        let mut wrapped_spans = vec![self.highlight_span("│ ".to_string())];
        wrapped_spans.append(&mut spans);
        wrapped_spans.push(self.highlight_span(format!("{fill} │")));

        let outer_bubble_padding =
            repeat_from_subtractions(" ", vec![self.window_max_width, formatted_line_length]);

        if self.alignment == BubbleAlignment::Left {
            wrapped_spans.push(Span::from(outer_bubble_padding));
            return Line::from(wrapped_spans);
        }

        let mut line_spans = vec![Span::from(outer_bubble_padding)];
        line_spans.extend(wrapped_spans);

        return Line::from(line_spans);
    }

    fn get_max_line_length(&self) -> usize {
        let style_config = Bubble::style_config();
        // Add a minimum 4% of padding on the side.
        let min_bubble_padding_length = ((self.window_max_width as f32
            * style_config.outer_padding_percentage)
            .ceil()) as usize;

        // Border elements + minimum bubble padding.
        let line_border_width = style_config.border_elements_length + min_bubble_padding_length;

        let mut max_line_length = self
            .message
            .text
            .lines()
            .map(|line| {
                return line.len();
            })
            .max()
            .unwrap();

        if max_line_length > (self.window_max_width - line_border_width) {
            max_line_length = self.window_max_width - line_border_width;
        }

        let username = &self.message.author.to_string();
        if max_line_length < username.len() {
            max_line_length = username.len();
        }

        return max_line_length;
    }

    fn wrap_lines_in_buddle(&self, lines: Vec<Line<'a>>, max_line_length: usize) -> Vec<Line<'a>> {
        // Add 2 for the vertical bars.
        let inner_bar = ["─"].repeat(max_line_length + 2).join("");
        let top_left_border = "╭";
        let mut top_bar = format!("{top_left_border}{inner_bar}╮");
        let bottom_bar = format!("╰{inner_bar}╯");
        let bar_bubble_padding = repeat_from_subtractions(
            " ",
            vec![
                self.window_max_width,
                max_line_length,
                Bubble::style_config().bubble_padding,
            ],
        );

        let username = &self.message.author.to_string();

        if self.alignment == BubbleAlignment::Left {
            let top_replace = ["─"].repeat(username.len()).join("");
            top_bar = top_bar.replace(
                format!("{top_left_border}{top_replace}").as_str(),
                format!("{top_left_border}{username}").as_str(),
            );

            let mut res = vec![self.highlight_line(format!("{top_bar}{bar_bubble_padding}"))];
            res.extend(lines);
            res.push(self.highlight_line(format!("{bottom_bar}{bar_bubble_padding}")));
            return res;
        } else {
            let top_replace = ["─"].repeat(username.len()).join("");
            top_bar = top_bar.replace(
                format!("{top_left_border}{top_replace}").as_str(),
                format!("{top_left_border}{username}").as_str(),
            );

            let mut res = vec![self.highlight_line(format!("{bar_bubble_padding}{top_bar}"))];
            res.extend(lines);
            res.push(self.highlight_line(format!("{bar_bubble_padding}{bottom_bar}")));
            return res;
        }
    }

    fn highlight_span(&self, text: String) -> Span<'a> {
        if self.message.message_type() == MessageType::Error {
            return Span::styled(
                text,
                Style {
                    fg: Some(Color::Red),
                    ..Style::default()
                },
            );
        } else if self.message.author == Author::Oatmeal {
            return Span::styled(
                text,
                Style {
                    fg: Some(Color::Rgb(138, 85, 63)), // Brown
                    ..Style::default()
                },
            );
        }

        return Span::from(text);
    }

    fn highlight_line(&self, text: String) -> Line<'a> {
        return Line::from(self.highlight_span(text));
    }
}
