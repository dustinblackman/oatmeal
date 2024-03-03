use super::EditorContext;

#[test]
fn it_renders_with_no_code() {
    let context = EditorContext {
        file_path: "file.rs".to_string(),
        language: "rust".to_string(),
        code: "".to_string(),
        start_line: 0,
        end_line: None,
    };

    insta::assert_snapshot!(context.format(), @"File: file.rs");
}

#[test]
fn it_renders_with_file_path() {
    let context = EditorContext {
        file_path: "file.rs".to_string(),
        language: "rust".to_string(),
        code: "let x = 5;".to_string(),
        start_line: 0,
        end_line: None,
    };

    insta::assert_snapshot!(context.format(), @"File: file.rs");
}

#[test]
fn it_renders_with_code() {
    let context = EditorContext {
        file_path: "file.rs".to_string(),
        language: "rust".to_string(),
        code: "let x = 5;".to_string(),
        start_line: 0,
        end_line: Some(1),
    };

    insta::assert_snapshot!(context.format(), @r###"
    File: file.rs

    ```rust
    let x = 5;
    ```
    "###);
}
