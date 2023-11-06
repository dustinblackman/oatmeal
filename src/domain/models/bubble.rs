#[cfg(test)]
#[path = "bubble_test.rs"]
mod tests;

use once_cell::sync::Lazy;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;

use super::Author;
use super::Message;
use super::MessageType;

static SYNTAX: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

#[derive(PartialEq, Eq)]
pub enum BubbleAlignment {
    Left,
    Right,
}

pub struct Bubble {
    message: Message,
}

// TODO this has gotten out of hand. Refactor.
impl<'a> Bubble {
    pub fn new(message: Message) -> Bubble {
        return Bubble { message };
    }

    pub fn as_lines(
        &self,
        alignment: BubbleAlignment,
        theme: &Theme,
        window_max_width: u16,
        total_codeblock_counter: usize,
    ) -> Vec<Line<'a>> {
        // Lazy defaults
        let syntax = find_syntax("json").unwrap();
        let mut highlight = HighlightLines::new(syntax, theme);

        // Add a minimum 4% of padding on the side.
        let min_bubble_padding_length = ((window_max_width as f32 * 0.04).ceil()) as usize;

        // left border + left padding + (text, not counted) + right padding + right
        // border + scrollbar. And then minimum bubble padding.
        let line_border_width = 5 + min_bubble_padding_length;

        let message_lines = self
            .message
            .as_string_lines(window_max_width - line_border_width as u16);

        let username = &self.message.author_formatted;
        let mut max_line_length = message_lines
            .iter()
            .map(|line| {
                return line.len();
            })
            .max()
            .unwrap();
        if max_line_length < username.len() {
            max_line_length = username.len();
        }

        let mut in_codeblock = false;
        let mut codeblock_count = 0;
        let mut lines: Vec<Line> = vec![];

        for line in message_lines {
            let (formatted_line, mut spans) = self.format_line(line.to_string(), max_line_length);
            let bubble_padding = [" "]
                .repeat(window_max_width as usize - formatted_line.len())
                .join("");

            if in_codeblock {
                let highlighted_spans: Vec<Span> = highlight
                    .highlight_line(&line, &SYNTAX)
                    .unwrap()
                    .iter()
                    .map(|segment| {
                        let (style, content) = segment;

                        return Span::styled(
                            content.to_string(),
                            Style {
                                fg: translate_colour(style.foreground),
                                ..Style::default()
                            },
                        );
                    })
                    .collect();

                spans = self
                    .format_spans(line.to_string(), max_line_length, highlighted_spans)
                    .1;
            }

            if line.trim().starts_with("```") {
                let lang = line.trim().replace("```", "");
                if let Some(syntax) = SYNTAX.find_syntax_by_token(&lang) {
                    highlight = HighlightLines::new(syntax, theme);
                    in_codeblock = true;

                    codeblock_count += 1;
                    spans = self
                        .format_spans(
                            format!("{line} ({})", total_codeblock_counter + codeblock_count),
                            max_line_length,
                            vec![
                                Span::from(line),
                                Span::styled(
                                    format!(" ({})", total_codeblock_counter + codeblock_count),
                                    Style {
                                        fg: Some(Color::White),
                                        ..Style::default()
                                    },
                                ),
                            ],
                        )
                        .1;
                } else {
                    in_codeblock = false;
                }
            }

            if alignment == BubbleAlignment::Left {
                spans.push(Span::from(bubble_padding));
                lines.push(Line::from(spans));
            } else {
                let mut res = vec![Span::from(bubble_padding)];
                res.extend(spans);
                lines.push(Line::from(res));
            }
        }

        // Add 2 for the vertical bars.
        let inner_bar = ["─"].repeat(max_line_length + 2).join("");
        let top_left_border = "╭";
        let mut top_bar = format!("{top_left_border}{inner_bar}╮");
        let bottom_bar = format!("╰{inner_bar}╯");
        let bar_bubble_padding = [" "]
            // TODO WTF is 8?
            .repeat(window_max_width as usize - max_line_length - 8)
            .join("");

        if alignment == BubbleAlignment::Left {
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
        line: String,
        max_line_length: usize,
        mut spans: Vec<Span<'a>>,
    ) -> (String, Vec<Span<'a>>) {
        let fill = [" "].repeat(max_line_length - line.len()).join("");
        let formatted_line = format!("│ {line}{fill} │");

        let mut spans_res = vec![self.highlight_span("│ ".to_string())];
        spans_res.append(&mut spans);
        spans_res.push(self.highlight_span(format!("{fill} │").to_string()));
        return (formatted_line, spans_res);
    }

    fn format_line(&self, line: String, max_line_length: usize) -> (String, Vec<Span<'a>>) {
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

fn translate_colour(syntect_color: syntect::highlighting::Color) -> Option<Color> {
    match syntect_color {
        syntect::highlighting::Color { r, g, b, a } if a > 0 => return Some(Color::Rgb(r, g, b)),
        _ => return None,
    }
}

fn find_syntax(name: &str) -> Option<&SyntaxReference> {
    if name == "typescript" {
        return SYNTAX.find_syntax_by_extension("javascript");
    }
    return SYNTAX.find_syntax_by_extension(name);
}
