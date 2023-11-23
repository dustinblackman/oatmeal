use super::SlashCommand;

#[test]
fn it_parse_empty_string() {
    let text = "";
    assert!(SlashCommand::parse(text).is_none());
}
#[test]
fn it_parse_space_only() {
    let text = " ";
    assert!(SlashCommand::parse(text).is_none());
}
#[test]
fn it_parse_single_slash() {
    let text = "/";
    assert!(SlashCommand::parse(text).is_none());
}
#[test]
fn it_parse_invalid_prefix() {
    let text = "!q";
    assert!(SlashCommand::parse(text).is_none());
}
#[test]
fn it_parse_valid_prefix() {
    let text = "/q";
    let cmd = SlashCommand::parse(text);
    assert!(cmd.is_some());
    assert_eq!(cmd.unwrap().command, "/q");
}

#[test]
fn it_is_short_quit() {
    let cmd = SlashCommand::parse("/q").unwrap();
    assert!(cmd.is_quit());
}
#[test]
fn it_is_quit() {
    let cmd = SlashCommand::parse("/quit").unwrap();
    assert!(cmd.is_quit());
}
#[test]
fn it_is_exit() {
    let cmd = SlashCommand::parse("/exit").unwrap();
    assert!(cmd.is_quit());
}
#[test]
fn it_is_not_is_quit() {
    let cmd = SlashCommand::parse("/ml").unwrap();
    assert!(!cmd.is_quit());
}

#[test]
fn it_is_short_model_list() {
    let cmd = SlashCommand::parse("/ml").unwrap();
    assert!(cmd.is_model_list());
}
#[test]
fn it_is_model_list() {
    let cmd = SlashCommand::parse("/modellist").unwrap();
    assert!(cmd.is_model_list());
}
#[test]
fn it_is_model_list_typo() {
    let cmd = SlashCommand::parse("/modelist").unwrap();
    assert!(cmd.is_model_list());
}
#[test]
fn it_is_not_model_list() {
    let cmd = SlashCommand::parse("/m").unwrap();
    assert!(!cmd.is_model_list());
}

#[test]
fn it_is_short_model_set() {
    let cmd = SlashCommand::parse("/m").unwrap();
    assert!(cmd.is_model_set());
}
#[test]
fn it_is_model_set() {
    let cmd = SlashCommand::parse("/model").unwrap();
    assert!(cmd.is_model_set());
}
#[test]
fn it_is_not_is_model_set() {
    let cmd = SlashCommand::parse("/ml").unwrap();
    assert!(!cmd.is_model_set());
}

#[test]
fn it_is_short_append_code_block() {
    let cmd = SlashCommand::parse("/a").unwrap();
    assert!(cmd.is_append_code_block());
}
#[test]
fn it_is_append_code_block() {
    let cmd = SlashCommand::parse("/append").unwrap();
    assert!(cmd.is_append_code_block());
}
#[test]
fn it_is_not_append_code_block() {
    let cmd = SlashCommand::parse("/ml").unwrap();
    assert!(!cmd.is_append_code_block());
}

#[test]
fn it_is_short_replace_code_block() {
    let cmd = SlashCommand::parse("/r").unwrap();
    assert!(cmd.is_replace_code_block());
}
#[test]
fn it_is_replace_code_block() {
    let cmd = SlashCommand::parse("/replace").unwrap();
    assert!(cmd.is_replace_code_block());
}
#[test]
fn it_is_not_replace_code_block() {
    let cmd = SlashCommand::parse("/ml").unwrap();
    assert!(!cmd.is_replace_code_block());
}

#[test]
fn it_is_short_help() {
    let cmd = SlashCommand::parse("/h").unwrap();
    assert!(cmd.is_help());
}
#[test]
fn it_is_help() {
    let cmd = SlashCommand::parse("/help").unwrap();
    assert!(cmd.is_help());
}
#[test]
fn it_is_not_help() {
    let cmd = SlashCommand::parse("/ml").unwrap();
    assert!(!cmd.is_help());
}

#[test]
fn it_is_short_copy_chat() {
    let cmd = SlashCommand::parse("/c").unwrap();
    assert!(cmd.is_copy_chat());
}
#[test]
fn it_is_copy_chat() {
    let cmd = SlashCommand::parse("/copy").unwrap();
    assert!(cmd.is_copy_chat());
}
#[test]
fn it_is_not_copy() {
    let cmd = SlashCommand::parse("/copy 1").unwrap();
    assert!(!cmd.is_copy_chat());
}

#[test]
fn it_is_short_copy_code() {
    let cmd = SlashCommand::parse("/c 1").unwrap();
    assert!(cmd.is_copy_code_block());
}
#[test]
fn it_is_copy_code() {
    let cmd = SlashCommand::parse("/copy 1").unwrap();
    assert!(cmd.is_copy_code_block());
}
#[test]
fn it_is_not_copy_code() {
    let cmd = SlashCommand::parse("/copy").unwrap();
    assert!(!cmd.is_copy_code_block());
}
