//! Tests for the non-panicking `Expr::validate` entry point.

mod common;

use common::check_ints;
use runit::{Expr, Is};

#[test]
fn validate_ok_when_passing() {
    assert!(Is::equal_to(5).validate(&5).is_ok());
}

#[test]
fn validate_err_carries_the_explanation() {
    let e = Is::equal_to(5);
    let err = e.validate(&3).unwrap_err();
    assert_eq!(err.to_string(), "Expected 5, got 3");
}

#[test]
fn validate_agrees_with_check() {
    let e = Is::at_least(-2).and().at_most(4).or().equal_to(9);
    for x in common::INTS {
        assert_eq!(e.validate(&x).is_ok(), e.check(&x), "mismatch for {x}");
    }
    check_ints(e, |x| (-2..=4).contains(&x) || x == 9);
}

#[test]
fn validate_error_is_a_std_error() {
    fn parse_port(raw: &str) -> Result<u16, Box<dyn std::error::Error>> {
        let rule = Is::in_range(1024..=49151);
        let port: u16 = raw.parse()?;
        rule.validate(&port).map_err(|e| e.to_string())?;
        Ok(port)
    }

    assert_eq!(parse_port("8080").unwrap(), 8080);
    assert_eq!(
        parse_port("80").unwrap_err().to_string(),
        "Expected in range 1024..=49151, got 80"
    );
}

#[test]
fn validate_error_implements_error_without_static() {
    fn source_of(e: &dyn std::error::Error) -> String {
        e.to_string()
    }
    let e = Is::greater_than(0);
    let err = e.validate(&0).unwrap_err();
    assert_eq!(source_of(&err), "Expected greater than 0, got 0");
}
