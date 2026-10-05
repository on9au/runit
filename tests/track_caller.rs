//! `Assert::that` should report the caller's location, not its own.
//!
//! Lives in its own test binary with a single test because it swaps the global panic hook.

use std::panic;
use std::sync::{Arc, Mutex};

use runit::{Assert, Is};

#[test]
fn assert_panics_at_the_call_site() {
    let location = Arc::new(Mutex::new(None));
    let captured = location.clone();

    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        *captured.lock().unwrap() = info.location().map(|l| (l.file().to_string(), l.line()));
    }));

    let expected_line = line!() + 1;
    let result = panic::catch_unwind(|| Assert::that(&1, Is::equal_to(2)));

    panic::set_hook(previous);

    assert!(result.is_err());
    let (file, line) = location.lock().unwrap().clone().expect("no panic location");
    assert!(
        file.ends_with("track_caller.rs"),
        "panic reported in {file}"
    );
    assert_eq!(line, expected_line);
}
