use super::super::EditorContext;
use super::BackendPrompt;

#[test]
fn it_adds_default_system_prompt() {
    let mut prompt = BackendPrompt::new("Hello world".to_string(), "".to_string());
    prompt.append_chat_context(&None);

    insta::assert_snapshot!(prompt.text, @"Hello world. Add language to any code blocks.");
}

#[test]
fn it_adds_language_system_prompt() {
    let mut prompt = BackendPrompt::new("Hello world".to_string(), "".to_string());
    prompt.append_chat_context(&Some(EditorContext {
        file_path: "./test.rs".to_string(),
        language: "rust".to_string(),
        code: "".to_string(),
        start_line: 0,
        end_line: None,
    }));

    insta::assert_snapshot!(prompt.text, @"Hello world. The coding language is rust. Add language to any code blocks.");
}

#[test]
fn it_adds_language_and_code_system_prompt() {
    let mut prompt = BackendPrompt::new("Hello world".to_string(), "".to_string());
    prompt.append_chat_context(&Some(EditorContext {
        file_path: "./test.rs".to_string(),
        language: "rust".to_string(),
        code: "println!(\"Test!\")".to_string(),
        start_line: 0,
        end_line: None,
    }));

    insta::assert_snapshot!(prompt.text, @r###"
    Hello world. The coding language is rust. Add language to any code blocks. The code is the following:
    println!("Test!")
    "###);
}
