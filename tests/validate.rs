//! Tests for the non-panicking `Expr::validate` entry point.

mod common;

use common::check_ints;
use runit::{Expr, ExprExt, Is};

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

#[test]
fn validate_works_on_trait_objects() {
    let e = Is::equal_to(1);
    let rule: &dyn Expr<i32> = &e;
    assert!(rule.validate(&1).is_ok());
    assert_eq!(
        rule.validate(&2).unwrap_err().to_string(),
        "Expected 1, got 2"
    );

    let boxed: Box<dyn Expr<i32>> = Box::new(Is::at_least(0));
    assert!(boxed.validate(&0).is_ok());
    assert!(boxed.validate(&-1).is_err());

    let shared: &(dyn Expr<i32> + Send + Sync) = &Is::at_most(9);
    assert_eq!(
        shared.validate(&10).unwrap_err().to_string(),
        "Expected at most 9, got 10"
    );
}

#[test]
fn validate_works_on_trait_object_fields() {
    struct Rule<'r> {
        expr: &'r dyn Expr<i32>,
    }
    let rules = [
        Rule {
            expr: &Is::greater_than(0),
        },
        Rule {
            expr: &Is::in_range(1..=3),
        },
    ];
    let described: Vec<String> = rules
        .iter()
        .map(|r| r.expr.description().to_string())
        .collect();
    assert_eq!(described, ["greater than 0", "in range 1..=3"]);
    assert_eq!(
        rules[1].expr.validate(&7).unwrap_err().to_string(),
        "Expected in range 1..=3, got 7"
    );
}

#[test]
fn prelude_brings_everything_needed() {
    mod scoped {
        use runit::prelude::*;

        pub fn run() -> String {
            let rule: &dyn Expr<i32> = &Is::at_least(0);
            Assert::that(&1, Is::at_least(0));
            rule.validate(&-1).unwrap_err().to_string()
        }
    }
    assert_eq!(scoped::run(), "Expected at least 0, got -1");
}
