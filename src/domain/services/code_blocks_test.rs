use test_utils::codeblock_fixture;

use super::CodeBlocks;
use crate::domain::models::Author;
use crate::domain::models::Message;
use crate::domain::models::SlashCommand;

fn from_slash_command(cmd_str: &str) -> String {
    let messages = vec![
        Message::new(Author::Oatmeal, "Hi there!"),
        Message::new(Author::Oatmeal, codeblock_fixture()),
    ];
    let command = SlashCommand::parse(cmd_str).unwrap();

    let mut codeblocks = CodeBlocks::default();
    codeblocks.replace_from_messages(&messages);
    return codeblocks.blocks_from_slash_commands(&command);
}

#[test]
fn it_replaces_messages() {
    let messages = vec![
        Message::new(Author::Oatmeal, "Hi there!"),
        Message::new(Author::Oatmeal, codeblock_fixture()),
    ];

    let mut codeblocks = CodeBlocks::default();
    codeblocks.replace_from_messages(&messages);
    assert_eq!(codeblocks.codeblocks.len(), 3);
}

#[test]
fn it_provides_first_codeblock() {
    let res = from_slash_command("/a 1");
    insta::assert_snapshot!(res, @r###"
    fn print_numbers() {
        for i in 0..=0 {
            println!("{i}");
        }
    }
    "###);
}

#[test]
fn it_provides_last_codeblock() {
    let res = from_slash_command("/a");
    insta::assert_snapshot!(res, @r###"
    for i in range(11):
        print(i)
    "###);
}

#[test]
fn it_provides_first_second_codeblock() {
    let res = from_slash_command("/a 1,2");
    insta::assert_snapshot!(res, @r###"
    fn print_numbers() {
        for i in 0..=0 {
            println!("{i}");
        }
    }

    function printNumbers() {
        let numbers = [];
        for (let i = 0; i <= 10; i++) {
            numbers.push(i);
        }
        return numbers.join('\n');
    }
    "###);
}

#[test]
fn it_provides_first_second_third_codeblock() {
    let res = from_slash_command("/a 1..3");
    insta::assert_snapshot!(res, @r###"
    fn print_numbers() {
        for i in 0..=0 {
            println!("{i}");
        }
    }

    function printNumbers() {
        let numbers = [];
        for (let i = 0; i <= 10; i++) {
            numbers.push(i);
        }
        return numbers.join('\n');
    }

    for i in range(11):
        print(i)
    "###);
}
