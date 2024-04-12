#[cfg(test)]
#[path = "slash_commands_test.rs"]
mod tests;

pub struct SlashCommand {
    command: String,
    pub args: Vec<String>,
}

impl SlashCommand {
    pub fn parse(text: &str) -> Option<SlashCommand> {
        let mut args = text
            .trim()
            .split(' ')
            .map(|e| return e.to_string())
            .collect::<Vec<String>>();
        let prefix = args[0].to_string();
        args.remove(0);

        let cmd = SlashCommand {
            command: prefix,
            args,
        };
        if cmd.is_quit()
            || cmd.is_model_list()
            || cmd.is_model_set()
            || cmd.is_append_code_block()
            || cmd.is_replace_code_block()
            || cmd.is_copy_code_block()
            || cmd.is_copy_chat()
            || cmd.is_help()
            || cmd.is_edit_prompt()
        {
            return Some(cmd);
        }

        return None;
    }

    pub fn is_quit(&self) -> bool {
        return ["/q", "/quit", "/exit"].contains(&self.command.as_str());
    }

    pub fn is_model_list(&self) -> bool {
        return ["/ml", "/modellist", "/modelist"].contains(&self.command.as_str());
    }

    pub fn is_model_set(&self) -> bool {
        return ["/m", "/model"].contains(&self.command.as_str());
    }

    pub fn is_append_code_block(&self) -> bool {
        return ["/a", "/append"].contains(&self.command.as_str());
    }

    pub fn is_replace_code_block(&self) -> bool {
        return ["/r", "/replace"].contains(&self.command.as_str());
    }

    pub fn is_copy_code_block(&self) -> bool {
        return ["/c", "/copy"].contains(&self.command.as_str()) && !self.args.is_empty();
    }

    pub fn is_copy_chat(&self) -> bool {
        return ["/c", "/copy"].contains(&self.command.as_str()) && self.args.is_empty();
    }

    pub fn is_help(&self) -> bool {
        return ["/h", "/help"].contains(&self.command.as_str());
    }
    pub fn is_edit_prompt(&self) -> bool {
        return ["/e", "/edit"].contains(&self.command.as_str());
    }
}
