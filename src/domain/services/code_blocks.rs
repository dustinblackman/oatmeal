use anyhow::bail;
use anyhow::Result;

use crate::domain::models::Message;
use crate::domain::models::SlashCommand;

#[cfg(test)]
#[path = "code_blocks_test.rs"]
mod tests;

#[derive(Default)]
pub struct CodeBlocks {
    codeblocks: Vec<String>,
}

impl CodeBlocks {
    pub fn replace_from_messages(&mut self, messages: &[Message]) {
        self.codeblocks = messages
            .iter()
            .flat_map(|msg| {
                return msg.codeblocks();
            })
            .collect();
    }

    pub fn blocks_from_slash_commands(&self, command: &SlashCommand) -> Result<String> {
        if self.codeblocks.is_empty() {
            return Ok("".to_string());
        }

        let args = command
            .args
            .iter()
            .map(|e| return e.trim().to_string())
            .filter(|e| return !e.is_empty())
            .collect::<Vec<String>>();

        if args.is_empty() {
            return Ok(self.codeblocks.last().unwrap().to_string());
        }

        let mut indexes = vec![];
        for arg in args.iter() {
            for e in arg.split(',') {
                let trimmed = arg.trim();
                if trimmed.contains("..") {
                    let split = trimmed.split("..").collect::<Vec<&str>>();
                    let first = self.validate_index(split[0].parse::<usize>()? - 1)?;
                    let last = self.validate_index(split[1].parse::<usize>()?)?;

                    indexes.extend_from_slice(&(first..last).collect::<Vec<usize>>())
                } else {
                    indexes.push(self.validate_index(e.parse::<usize>()? - 1)?)
                }
            }
        }

        let res = indexes
            .iter()
            .map(|idx| return self.codeblocks[*idx].to_string())
            .collect::<Vec<String>>()
            .join("\n\n");

        return Ok(res);
    }

    fn validate_index(&self, idx: usize) -> Result<usize> {
        if idx > self.codeblocks.len() {
            bail!(format!("{idx} is out of bounds."))
        }
        return Ok(idx);
    }
}
