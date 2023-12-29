use test_utils::codeblock_fixture;

use super::Author;
use super::Message;
use super::MessageType;

#[test]
fn it_executes_new() {
    let msg = Message::new(Author::Oatmeal, "Hi there!");
    assert_eq!(msg.author, Author::Oatmeal);
    assert_eq!(msg.author.to_string(), "Oatmeal");
    assert_eq!(msg.text, "Hi there!".to_string());
    assert_eq!(msg.mtype, MessageType::Normal);
}

#[test]
fn it_executes_new_replacing_tabs() {
    let msg = Message::new(Author::Oatmeal, "\t\tHi there!");
    assert_eq!(msg.author, Author::Oatmeal);
    assert_eq!(msg.author.to_string(), "Oatmeal");
    assert_eq!(msg.text, "    Hi there!".to_string());
    assert_eq!(msg.mtype, MessageType::Normal);
}

#[test]
fn it_executes_new_with_type() {
    let msg = Message::new_with_type(Author::Oatmeal, MessageType::Error, "It broke!");
    assert_eq!(msg.author, Author::Oatmeal);
    assert_eq!(msg.author.to_string(), "Oatmeal");
    assert_eq!(msg.text, "It broke!".to_string());
    assert_eq!(msg.mtype, MessageType::Error);
}

#[test]
fn it_executes_new_with_type_replacing_tabs() {
    let msg = Message::new_with_type(Author::Oatmeal, MessageType::Error, "\t\tIt broke!");
    assert_eq!(msg.author, Author::Oatmeal);
    assert_eq!(msg.author.to_string(), "Oatmeal");
    assert_eq!(msg.text, "    It broke!".to_string());
    assert_eq!(msg.mtype, MessageType::Error);
}

#[test]
fn it_executes_message_type() {
    let msg = Message::new_with_type(Author::Oatmeal, MessageType::Error, "It broke!");
    assert_eq!(msg.message_type(), MessageType::Error);
}

#[test]
fn it_executes_append() {
    let mut msg = Message::new(Author::Oatmeal, "Hi there!");
    msg.append(" It's me!");
    assert_eq!(msg.text, "Hi there! It's me!");
}

#[test]
fn it_executes_append_with_tabs() {
    let mut msg = Message::new(Author::Oatmeal, "Hi there!");
    msg.append("\tIt's me!");
    assert_eq!(msg.text, "Hi there!  It's me!");
}

#[test]
fn it_executes_codeblocks() {
    let msg = Message::new(Author::Oatmeal, codeblock_fixture());
    let codeblocks = msg.codeblocks();

    assert_eq!(codeblocks.len(), 4);
    insta::assert_snapshot!(codeblocks[0], @r###"
    fn print_numbers() {
        for i in 0..=0 {
            println!("{i}");
        }
    }
    "###);

    insta::assert_snapshot!(codeblocks[1], @r###"
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

    insta::assert_snapshot!(codeblocks[2], @"abc123");

    insta::assert_snapshot!(codeblocks[3], @r###"
    for i in range(11):
        print(i)
    "###);
}
