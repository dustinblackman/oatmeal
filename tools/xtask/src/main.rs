mod readme;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let last = args.last().unwrap();
    if last == "update-readme" {
        readme::update();
    } else {
        eprintln!("ERROR: No task selected");
        process::exit(1);
    }
}
