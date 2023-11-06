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

    pub fn blocks_from_slash_commands(&self, command: &SlashCommand) -> String {
        if self.codeblocks.is_empty() {
            return "".to_string();
        }

        let args = command
            .args
            .iter()
            .map(|e| return e.trim().to_string())
            .filter(|e| return !e.is_empty())
            .collect::<Vec<String>>();

        if args.is_empty() {
            return self.codeblocks.last().unwrap().to_string();
        }

        return args
            .iter()
            .flat_map(|arg| {
                return arg
                    .split(',')
                    .map(|e| {
                        let trimmed = arg.trim();
                        if trimmed.contains("..") {
                            let split = trimmed.split("..").collect::<Vec<&str>>();
                            let first = split[0].parse::<usize>().unwrap() - 1;
                            let last = split[1].parse::<usize>().unwrap();
                            return (first..last).collect::<Vec<usize>>();
                        }

                        return vec![e.parse::<usize>().unwrap() - 1];
                    })
                    .collect::<Vec<Vec<usize>>>();
            })
            .flatten()
            .map(|idx| return self.codeblocks[idx].to_string())
            .collect::<Vec<String>>()
            .join("\n\n");
    }
}
