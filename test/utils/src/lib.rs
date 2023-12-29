use std::env;

pub fn insta_snapshot<F: FnOnce()>(f: F) {
    let mut settings = insta::Settings::clone_current();
    let snapshot_path = env::current_dir().unwrap().join("./test/snapshots");
    settings.set_snapshot_path(snapshot_path);
    settings.bind(f);
}

pub fn codeblock_fixture() -> &'static str {
    return r#"
Here's how to print in Rust.

```rust
fn print_numbers() {
    for i in 0..=0 {
        println!("{i}");
    }
}
```

And in Javascript.

```javascript
// Hello World.

// This is a really long line that pushes the boundaries of 50 characters across the screen, resulting in a code comment block where the line is wrapped to the next line. Cool right?
function printNumbers() {
    let numbers = [];
    for (let i = 0; i <= 10; i++) {
        numbers.push(i);
    }
    return numbers.join('\n');
}
```

This is a markdown codeblock that has no language. We count it as well incase an LLM doesn't attach a language.

```
abc123
```

Let's do Python as well!

```python
for i in range(11):
    print(i)
```

That's it!
"#
    .trim();
}
