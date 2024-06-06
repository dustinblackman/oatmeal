#[cfg(test)]
#[path = "message_test.rs"]
mod tests;
use serde::Deserialize;
use serde::Serialize;

use super::Author;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Normal,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub author: Author,
    pub text: String,
    mtype: MessageType,
}

impl Message {
    pub fn new(author: Author, text: &str) -> Message {
        return Message {
            author: author.clone(),
            text: text.to_string().replace('\t', "  "),
            mtype: MessageType::Normal,
        };
    }

    pub fn new_with_type(author: Author, mtype: MessageType, text: &str) -> Message {
        return Message {
            author: author.clone(),
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
