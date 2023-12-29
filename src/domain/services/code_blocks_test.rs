use anyhow::Result;
use test_utils::codeblock_fixture;

use super::CodeBlocks;
use crate::domain::models::Author;
use crate::domain::models::Message;
use crate::domain::models::SlashCommand;

fn from_slash_command(cmd_str: &str) -> Result<String> {
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
    assert_eq!(codeblocks.codeblocks.len(), 4);
}

#[test]
fn it_provides_first_codeblock() {
    let res = from_slash_command("/a 1").unwrap();
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
    let res = from_slash_command("/a").unwrap();
    insta::assert_snapshot!(res, @r###"
    for i in range(11):
        print(i)
    "###);
}

#[test]
fn it_provides_first_second_codeblock() {
    let res = from_slash_command("/a 1,2").unwrap();
    insta::assert_snapshot!(res, @r###"
    fn print_numbers() {
        for i in 0..=0 {
            println!("{i}");
        }
    }

    // Hello World.

    // This is a really long line that pushes the boundaries of 50 characters across the screen, resulting in a code comment block where the line is wrapped to the next line. Cool right?
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
    let res = from_slash_command("/a 1..3").unwrap();
    insta::assert_snapshot!(res, @r###"
    fn print_numbers() {
        for i in 0..=0 {
            println!("{i}");
        }
    }

    // Hello World.

    // This is a really long line that pushes the boundaries of 50 characters across the screen, resulting in a code comment block where the line is wrapped to the next line. Cool right?
    function printNumbers() {
        let numbers = [];
        for (let i = 0; i <= 10; i++) {
            numbers.push(i);
        }
        return numbers.join('\n');
    }

    abc123
    "###);
}

#[test]
fn it_throws_an_error_on_invalid_index() {
    let res = from_slash_command("/a 1010101").unwrap_err().to_string();
    insta::assert_snapshot!(res, @"Code block index 1010101 is not valid");
}
