//! Tests for comparison and range expressions.

mod common;

use common::{EvalExt, check_ints};
use runit::{AtLeast, AtMost, GreaterThan, InRange, Is, LessThan};

#[test]
fn comparisons_match_their_operators() {
    check_ints(Is::greater_than(2), |x| x > 2);
    check_ints(Is::less_than(2), |x| x < 2);
    check_ints(Is::at_least(2), |x| x >= 2);
    check_ints(Is::at_most(2), |x| x <= 2);
}

#[test]
fn negated_comparisons() {
    check_ints(Is::not().greater_than(2), |x| x <= 2);
    check_ints(Is::not().at_least(0), |x| x < 0);
}

#[test]
fn ranges_of_every_shape() {
    check_ints(Is::in_range(-3..4), |x| (-3..4).contains(&x));
    check_ints(Is::in_range(-3..=4), |x| (-3..=4).contains(&x));
    check_ints(Is::in_range(..0), |x| x < 0);
    check_ints(Is::in_range(5..), |x| x >= 5);
    check_ints(Is::in_range(..), |_| true);
}

#[test]
fn comparisons_chain_with_and_or() {
    check_ints(
        Is::greater_than(-5).and().less_than(5).or().equal_to(8),
        |x| (-5 < x && x < 5) || x == 8,
    );
}

#[test]
fn comparison_messages() {
    assert_eq!(
        GreaterThan { value: 5 }.eval(&3).message,
        "Expected greater than 5, got 3"
    );
    assert_eq!(
        LessThan { value: 5 }.eval(&7).message,
        "Expected less than 5, got 7"
    );
    assert_eq!(
        AtLeast { value: 5 }.eval(&3).message,
        "Expected at least 5, got 3"
    );
    assert_eq!(
        AtMost { value: 5 }.eval(&7).message,
        "Expected at most 5, got 7"
    );
    assert_eq!(
        InRange { range: 1..=3 }.eval(&7).message,
        "Expected in range 1..=3, got 7"
    );
}

#[test]
fn comparison_descriptions() {
    let e = Is::at_least(1).and().at_most(3).or().in_range(10..20);
    assert_eq!(
        EvalExt::<i32>::describe_string(&e),
        "at least 1 and at most 3 or in range 10..20"
    );
}

#[test]
fn comparisons_work_on_floats_and_strings() {
    assert!(Is::greater_than(0.5).eval(&0.75).pass);
    assert!(!Is::greater_than(f64::NAN).eval(&1.0).pass);
    assert!(Is::less_than("b").eval(&"a").pass);
    assert!(Is::in_range('a'..='z').eval(&'q').pass);
}
