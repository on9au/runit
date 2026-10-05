//! Tests for string expressions.

mod common;

use common::EvalExt;
use runit::{ContainsStr, EndsWith, Is, StartsWith};

#[test]
fn string_predicates() {
    assert!(Is::starts_with("ab").eval(&"abc").pass);
    assert!(!Is::starts_with("bc").eval(&"abc").pass);
    assert!(Is::ends_with("bc").eval(&"abc").pass);
    assert!(!Is::ends_with("ab").eval(&"abc").pass);
    assert!(Is::contains_str("b").eval(&"abc").pass);
    assert!(!Is::contains_str("z").eval(&"abc").pass);
}

#[test]
fn string_predicates_accept_owned_and_unsized_strings() {
    assert!(Is::starts_with("he").eval(&String::from("hello")).pass);
    assert!(Is::ends_with(String::from("lo")).eval(&"hello").pass);
    let r = <StartsWith<&str> as EvalExt<str>>::eval(&StartsWith { pattern: "x" }, "hello");
    assert!(!r.pass);
}

#[test]
fn empty_pattern_always_matches() {
    assert!(Is::starts_with("").eval(&"").pass);
    assert!(Is::contains_str("").eval(&"abc").pass);
}

#[test]
fn string_messages() {
    assert_eq!(
        StartsWith { pattern: "x" }.eval(&"abc").message,
        r#"Expected string starting with "x", got "abc""#
    );
    assert_eq!(
        EndsWith { pattern: "x" }.eval(&"abc").message,
        r#"Expected string ending with "x", got "abc""#
    );
    assert_eq!(
        ContainsStr { pattern: "x" }.eval(&"a\"b").message,
        r#"Expected string containing "x", got "a\"b""#
    );
}

#[test]
fn string_descriptions_chain() {
    let e = Is::starts_with("http://")
        .or()
        .starts_with("https://")
        .and()
        .not()
        .ends_with("/");
    assert_eq!(
        EvalExt::<&str>::describe_string(&e),
        r#"starting with "http://" or starting with "https://" and not ending with "/""#
    );
    assert!(e.eval(&"https://a.io").pass);
    assert!(!e.eval(&"https://a.io/").pass);
    assert!(!e.eval(&"ftp://a.io").pass);
}
