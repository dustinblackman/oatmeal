use anyhow::Result;
use test_utils::codeblock_fixture;
use test_utils::insta_snapshot;

use super::Bubble;
use super::BubbleAlignment;
use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::models::Author;
use crate::domain::models::Message;
use crate::domain::services::Themes;

fn create_lines(
    author: Author,
    alignment: BubbleAlignment,
    codeblock_count: usize,
    text: &str,
) -> Result<String> {
    Config::set(ConfigKey::Username, "testuser");
    Config::set(ConfigKey::Model, "model-1");

    let message = Message::new(author, text);
    let theme = Themes::get("base16-seti", "")?;
    let lines = Bubble::new(&message, alignment, 50, codeblock_count).as_lines(&theme);
    let lines_str = lines
        .iter()
        .map(|line| {
            return line
                .spans
                .iter()
                .map(|span| {
                    return span.content.to_string();
                })
                .collect::<Vec<String>>()
                .join("");
        })
        .collect::<Vec<String>>()
        .join("\n");

    return Ok(lines_str);
}

#[test]
fn it_creates_author_oatmeal_text() -> Result<()> {
    let lines_str = create_lines(Author::Oatmeal, BubbleAlignment::Left, 0, "Hi there!")?;
    insta::assert_snapshot!(lines_str, @r###"
    ╭Oatmeal────╮                                 
    │ Hi there! │                                 
    ╰───────────╯                                 
    "###);

    return Ok(());
}

#[test]
fn it_creates_author_oatmeal_text_long() -> Result<()> {
    let lines_str = create_lines(Author::Oatmeal, BubbleAlignment::Left, 0, "Hi there! This is a really long line that pushes the boundaries of 50 characters across the screen, resulting in a bubble where the line is wrapped to the next line. Cool right?")?;
    insta::assert_snapshot!(lines_str, @r###"
    ╭Oatmeal──────────────────────────────────────╮
    │ Hi there! This is a really long line that   │
    │ pushes the boundaries of 50 characters      │
    │ across the screen, resulting in a bubble    │
    │ where the line is wrapped to the next line. │
    │ Cool right?                                 │
    ╰─────────────────────────────────────────────╯
    "###);

    return Ok(());
}

#[test]
fn it_creates_author_model_text_code() -> Result<()> {
    let text = r#"
Here's how to print in Rust.

```rust
fn print_numbers() {
    for i in 0..=0 {
        println!("{i}");
    }
}
```"#
        .trim();
    let lines_str = create_lines(Author::Model, BubbleAlignment::Left, 0, text)?;

    insta::assert_snapshot!(lines_str, @r###"
    ╭model-1───────────────────────╮              
    │ Here's how to print in Rust. │              
    │                              │              
    │ ```rust (1)                  │              
    │ fn print_numbers() {         │              
    │     for i in 0..=0 {         │              
    │         println!("{i}");     │              
    │     }                        │              
    │ }                            │              
    │ ```                          │              
    ╰──────────────────────────────╯              
    "###);
    return Ok(());
}

#[test]
fn it_creates_author_model_text_code_multiple_blocks() -> Result<()> {
    let lines_str = create_lines(Author::Model, BubbleAlignment::Left, 9, codeblock_fixture())?;
    insta_snapshot(|| {
        insta::assert_toml_snapshot!(lines_str);
    });

    return Ok(());
}

#[test]
fn it_creates_author_model_text() -> Result<()> {
    let lines_str = create_lines(Author::Model, BubbleAlignment::Left, 0, "Hi there!")?;
    insta::assert_snapshot!(lines_str, @r###"
    ╭model-1────╮                                 
    │ Hi there! │                                 
    ╰───────────╯                                 
    "###);

    return Ok(());
}

#[test]
fn it_creates_author_user_text() -> Result<()> {
    let lines_str = create_lines(Author::User, BubbleAlignment::Right, 0, "Hi there!")?;
    insta_snapshot(|| {
        insta::assert_toml_snapshot!(lines_str);
    });

    return Ok(());
}
