#[cfg(test)]
#[path = "message_test.rs"]
mod tests;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::config::Config;
use crate::config::ConfigKey;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Author {
    User,
    Oatmeal,
    Model,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Normal,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub author: Author,
    pub text: String,
    pub author_formatted: String,
    mtype: MessageType,
}

fn formatted_author(author: Author) -> String {
    return match author {
        Author::User => Config::get(ConfigKey::Username),
        Author::Oatmeal => "Oatmeal".to_string(),
        Author::Model => Config::get(ConfigKey::Model),
    };
}

impl Message {
    pub fn new(author: Author, text: &str) -> Message {
        return Message {
            author: author.clone(),
            author_formatted: formatted_author(author),
            text: text.to_string().replace('\t', "  "),
            mtype: MessageType::Normal,
        };
    }

    pub fn new_with_type(author: Author, mtype: MessageType, text: &str) -> Message {
        return Message {
            author: author.clone(),
            author_formatted: formatted_author(author),
            text: text.to_string().replace('\t', "  "),
            mtype,
        };
    }

    pub fn message_type(&self) -> MessageType {
        return self.mtype;
    }

    pub fn append(&mut self, text: &str) {
        self.text += &text.replace('\t', "  ");
    }

    pub fn as_string_lines(&self, line_max_width: u16) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        for full_line in self.text.split('\n') {
            // TODO may not need this.
            if full_line.trim().is_empty() {
                lines.push(" ".to_string());
                continue;
            }

            let mut char_count = 0;
            let mut current_lines: Vec<&str> = vec![];

            for word in full_line.split(' ') {
                if word.len() + char_count + 1 > line_max_width.into() {
                    lines.push(current_lines.join(" ").trim_end().to_string());
                    current_lines = vec![word];
                    char_count = word.len() + 1;
                } else {
                    current_lines.push(word);
                    char_count += word.len() + 1;
                }
            }
            if !current_lines.is_empty() {
                lines.push(current_lines.join(" ").trim_end().to_string());
            }
        }

        return lines;
    }

    pub fn codeblocks(&self) -> Vec<String> {
        let mut codeblocks: Vec<String> = vec![];
        let mut current_codeblock: Vec<&str> = vec![];
        let mut in_codeblock = false;

        for line in self.text.split('\n') {
            let trimmed = line.trim();
            if trimmed.starts_with("```") {
                if in_codeblock {
                    codeblocks.push(current_codeblock.join("\n"));
                    current_codeblock = vec![];
                    in_codeblock = false
                } else {
                    // Only count code blocks with a language attached to it.
                    if trimmed == "```" {
                        continue;
                    }
                    in_codeblock = true;
                }
                continue;
            }

            if in_codeblock {
                current_codeblock.push(line);
            }
        }

        return codeblocks;
    }
}
