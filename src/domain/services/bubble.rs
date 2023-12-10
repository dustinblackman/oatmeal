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

    pub fn style_confg() -> BubbleConfig {
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

        let (message_lines, max_line_length) = self.get_message_lines();

        for line in message_lines {
            let line_length = line.len();
            let (mut spans, formatted_line_length) =
                self.format_line(line.to_string(), max_line_length);

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
                    .format_spans(highlighted_spans, line_length, max_line_length)
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
                            format!(" ({})", self.codeblock_counter).len() + line_length,
                            max_line_length,
                        )
                        .0;
                } else {
                    in_codeblock = false;
                }
            }

            let bubble_padding =
                repeat_from_subtractions(" ", vec![self.window_max_width, formatted_line_length]);

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
        let min_bubble_padding_length = ((self.window_max_width as f32
            * Bubble::style_confg().outer_padding_percentage)
            .ceil()) as usize;

        // Border elements + minimum bubble padding.
        let line_border_width =
            Bubble::style_confg().border_elements_length + min_bubble_padding_length;

        let message_lines = self
            .message
            .as_string_lines(self.window_max_width - line_border_width);

        let mut max_line_length = message_lines
            .iter()
            .map(|line| {
                return line.len();
            })
            .max()
            .unwrap();

        let username = &self.message.author.to_string();
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
        let bar_bubble_padding = repeat_from_subtractions(
            " ",
            vec![
                self.window_max_width,
                max_line_length,
                Bubble::style_confg().bubble_padding,
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

    fn format_spans(
        &self,
        mut spans: Vec<Span<'a>>,
        line_str_len: usize,
        max_line_length: usize,
    ) -> (Vec<Span<'a>>, usize) {
        let fill = repeat_from_subtractions(" ", vec![max_line_length, line_str_len]);
        // 8 is the unicode character border + padding.
        let formatted_line_length = line_str_len + fill.len() + 8;

        let mut spans_res = vec![self.highlight_span("│ ".to_string())];
        spans_res.append(&mut spans);
        spans_res.push(self.highlight_span(format!("{fill} │").to_string()));
        return (spans_res, formatted_line_length);
    }

    fn format_line(&self, line: String, max_line_length: usize) -> (Vec<Span<'a>>, usize) {
        let line_len = line.len();
        return self.format_spans(vec![Span::from(line)], line_len, max_line_length);
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
