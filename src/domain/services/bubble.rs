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

pub struct Bubble {
    alignment: BubbleAlignment,
    message: Message,
    window_max_width: u16,
    codeblock_counter: usize,
}

impl<'a> Bubble {
    pub fn new(
        message: Message,
        alignment: BubbleAlignment,
        window_max_width: u16,
        codeblock_counter: usize,
    ) -> Bubble {
        return Bubble {
            alignment,
            message,
            window_max_width,
            codeblock_counter,
        };
    }

    pub fn as_lines(&mut self, theme: &Theme) -> Vec<Line<'a>> {
        // Lazy default
        let mut highlight = HighlightLines::new(Syntaxes::get("text"), theme);
        let mut in_codeblock = false;
        let mut lines: Vec<Line> = vec![];

        let (message_lines, max_line_length) = self.get_message_lines();

        for line in message_lines {
            let (mut spans, line_length) = self.format_line(line.to_string(), max_line_length);

            if in_codeblock {
                let highlighted_spans: Vec<Span> = highlight
                    .highlight_line(&line, &SYNTAX_SET)
                    .unwrap()
                    .iter()
                    .map(|segment| {
                        let (style, content) = segment;

                        return Span::styled(
                            content.to_string(),
                            Style {
                                fg: Syntaxes::translate_colour(style.foreground),
                                ..Style::default()
                            },
                        );
                    })
                    .collect();

                spans = self
                    .format_spans(line.to_string(), max_line_length, highlighted_spans)
                    .0;
            }

            if line.trim().starts_with("```") {
                let lang = line.trim().replace("```", "");
                let syntax = Syntaxes::get(&lang);
                if syntax.name.to_lowercase() != "plain text" {
                    highlight = HighlightLines::new(syntax, theme);
                    in_codeblock = true;

                    self.codeblock_counter += 1;
                    spans = self
                        .format_spans(
                            format!("{line} ({})", self.codeblock_counter),
                            max_line_length,
                            vec![
                                Span::from(line),
                                Span::styled(
                                    format!(" ({})", self.codeblock_counter),
                                    Style {
                                        fg: Some(Color::White),
                                        ..Style::default()
                                    },
                                ),
                            ],
                        )
                        .0;
                } else {
                    in_codeblock = false;
                }
            }

            let bubble_padding = [" "]
                .repeat(self.window_max_width as usize - line_length)
                .join("");

            if self.alignment == BubbleAlignment::Left {
                spans.push(Span::from(bubble_padding));
                lines.push(Line::from(spans));
            } else {
                let mut line_spans = vec![Span::from(bubble_padding)];
                line_spans.extend(spans);
                lines.push(Line::from(line_spans));
            }
        }

        return self.wrap_lines_in_buddle(lines, max_line_length);
    }

    fn get_message_lines(&self) -> (Vec<String>, usize) {
        // Add a minimum 4% of padding on the side.
        let min_bubble_padding_length = ((self.window_max_width as f32 * 0.04).ceil()) as usize;

        // left border + left padding + (text, not counted) + right padding + right
        // border + scrollbar. And then minimum bubble padding.
        let line_border_width = 5 + min_bubble_padding_length;

        let message_lines = self
            .message
            .as_string_lines(self.window_max_width - line_border_width as u16);

        let mut max_line_length = message_lines
            .iter()
            .map(|line| {
                return line.len();
            })
            .max()
            .unwrap();

        let username = &self.message.author_formatted;
        if max_line_length < username.len() {
            max_line_length = username.len();
        }

        return (message_lines, max_line_length);
    }

    fn wrap_lines_in_buddle(&self, lines: Vec<Line<'a>>, max_line_length: usize) -> Vec<Line<'a>> {
        // Add 2 for the vertical bars.
        let inner_bar = ["─"].repeat(max_line_length + 2).join("");
        let top_left_border = "╭";
        let mut top_bar = format!("{top_left_border}{inner_bar}╮");
        let bottom_bar = format!("╰{inner_bar}╯");
        let bar_bubble_padding = [" "]
            // TODO WTF is 8?
            .repeat(self.window_max_width as usize - max_line_length - 8)
            .join("");

        let username = &self.message.author_formatted;

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

    fn format_spans(
        &self,
        line_str: String,
        max_line_length: usize,
        mut spans: Vec<Span<'a>>,
    ) -> (Vec<Span<'a>>, usize) {
        let fill = [" "].repeat(max_line_length - line_str.len()).join("");
        let line_length = format!("│ {line_str}{fill} │").len();

        let mut spans_res = vec![self.highlight_span("│ ".to_string())];
        spans_res.append(&mut spans);
        spans_res.push(self.highlight_span(format!("{fill} │").to_string()));
        return (spans_res, line_length);
    }

    fn format_line(&self, line: String, max_line_length: usize) -> (Vec<Span<'a>>, usize) {
        return self.format_spans(
            line.to_string(),
            max_line_length,
            vec![Span::from(line.clone())],
        );
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
