use anyhow::Result;
use test_utils::codeblock_fixture;

use super::BubbleList;
use crate::domain::models::Author;
use crate::domain::models::Message;
use crate::domain::services::Themes;

#[test]
fn it_has_no_cached_lines() -> Result<()> {
    let theme = Themes::get("base16-seti", "")?;
    let bubble_list = BubbleList::new(theme);

    assert_eq!(bubble_list.cache.len(), 0);
    return Ok(());
}

#[test]
fn it_caches_lines() -> Result<()> {
    let theme = Themes::get("base16-seti", "")?;
    let messages = vec![
        Message::new(Author::Oatmeal, "Hi there!"),
        Message::new(Author::Oatmeal, codeblock_fixture()),
    ];

    let mut bubble_list = BubbleList::new(theme);
    bubble_list.set_messages(&messages, 50);

    assert_eq!(bubble_list.cache.len(), 2);
    return Ok(());
}

#[test]
fn it_returns_correct_length() -> Result<()> {
    let theme = Themes::get("base16-seti", "")?;
    let messages = vec![
        Message::new(Author::Oatmeal, "Hi there!"),
        Message::new(Author::Oatmeal, codeblock_fixture()),
    ];

    let mut bubble_list = BubbleList::new(theme);
    bubble_list.set_messages(&messages, 50);

    assert_eq!(bubble_list.len(), 50);
    return Ok(());
}
