//! Tests for length and collection expressions.

mod common;

use std::collections::{BTreeMap, BTreeSet, LinkedList, VecDeque};

use common::EvalExt;
use runit::{Expr, Is};

#[test]
fn empty_across_types() {
    assert!(Is::empty().eval(&"").pass);
    assert!(Is::empty().eval(&String::new()).pass);
    assert!(Is::empty().eval(&Vec::<i32>::new()).pass);
    assert!(Is::empty().eval(&[0u8; 0]).pass);
    assert!(Is::empty().eval(&BTreeMap::<i32, i32>::new()).pass);
    assert!(!Is::empty().eval(&vec![1]).pass);
    assert!(!Is::empty().eval(&"a").pass);
    assert!(Is::not().empty().eval(&VecDeque::from([1])).pass);
}

#[test]
fn empty_message_reports_length() {
    assert_eq!(
        Is::empty().eval(&vec![1, 2, 3]).message,
        "Expected empty, got length 3"
    );
}

#[test]
fn length_composes_with_any_usize_expr() {
    let e = Is::length(Is::in_range(2..=4));
    assert!(e.eval(&"abc").pass);
    assert!(!e.eval(&"a").pass);
    assert!(e.eval(&[1, 2]).pass);
    assert_eq!(
        e.eval(&"a").message,
        "Expected length in range 2..=4, got 1"
    );
}

#[test]
fn length_description_groups_compound_inner() {
    let e = Is::length(Is::equal_to(1).or().equal_to(3));
    assert_eq!(
        EvalExt::<&str>::describe_string(&e),
        "length (equal to 1 or equal to 3)"
    );
}

#[test]
fn contains_across_collections() {
    assert!(Is::contains(2).eval(&vec![1, 2, 3]).pass);
    assert!(!Is::contains(9).eval(&vec![1, 2, 3]).pass);
    assert!(Is::contains(2).eval(&[1, 2, 3]).pass);
    assert!(Is::contains(2).eval(&BTreeSet::from([2])).pass);
    assert!(Is::contains(2).eval(&LinkedList::from([2])).pass);
    assert!(
        Is::contains("b")
            .eval(&vec![String::from("a"), String::from("b")])
            .pass
    );
    assert_eq!(
        Is::contains(9).eval(&vec![1]).message,
        "Expected collection containing 9, but it did not"
    );
}

#[test]
fn contains_on_slices_and_references() {
    let v = vec![1, 2, 3];
    let s: &[i32] = &v;
    assert!(Is::contains(3).eval(&s).pass);
    let r = <runit::Contains<i32> as EvalExt<[i32]>>::eval(&runit::Contains { value: 3 }, s);
    assert!(r.pass);
}

#[test]
fn any_and_all() {
    let v = vec![2, 4, 7];
    assert!(Is::any(Is::greater_than(5)).eval(&v).pass);
    assert!(!Is::any(Is::greater_than(10)).eval(&v).pass);
    assert!(
        !Is::all(Is::satisfies("even", |x: &i32| x % 2 == 0))
            .eval(&v)
            .pass
    );
    assert!(Is::all(Is::less_than(10)).eval(&v).pass);
}

#[test]
fn all_is_vacuously_true_and_any_false_on_empty() {
    let v: Vec<i32> = vec![];
    assert!(Is::all(Is::equal_to(1)).eval(&v).pass);
    assert!(!Is::any(Is::equal_to(1)).eval(&v).pass);
}

#[test]
fn all_explains_the_first_failing_item() {
    let e = Is::all(Is::less_than(5));
    assert_eq!(
        e.eval(&vec![1, 7, 9]).message,
        "Item at index 1: Expected less than 5, got 7"
    );
}

#[test]
fn any_explains_with_description() {
    let e = Is::any(Is::equal_to(1).or().equal_to(2));
    assert_eq!(
        e.eval(&vec![5]).message,
        "Expected any item (equal to 1 or equal to 2), but none was"
    );
}

#[test]
fn nested_collections() {
    let grid = vec![vec![1, 2], vec![3]];
    let e = Is::all(Is::not().empty().and().length(Is::at_most(2)));
    assert!(e.check(&grid));
    assert!(!e.check(&vec![vec![1], vec![]]));
}
