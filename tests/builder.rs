//! Tests for the fluent `Is::...` builder and the `Query` expressions it produces.
//!
//! The builder is expected to give `and` higher precedence than `or`, i.e.
//! `a.or().b.and().c` means `a || (b && c)`.

mod common;

use common::{check_ints, check_options};
use runit::{Expr, Is, Negated, PendingAnd, PendingOr};

fn either(items: &[&str]) -> String {
    let mut out = String::from("Expected either:");
    for item in items {
        out.push_str("\n  - ");
        out.push_str(item);
    }
    out
}

// ---------------------------------------------------------------- single atoms

#[test]
fn is_none() {
    check_options(Is::none(), |x| x.is_none());
}

#[test]
fn is_not_none() {
    check_options(Is::not().none(), |x| x.is_some());
}

#[test]
fn is_equal_to() {
    check_ints(Is::equal_to(3), |x| x == 3);
}

#[test]
fn is_not_equal_to() {
    check_ints(Is::not().equal_to(3), |x| x != 3);
}

#[test]
fn is_equal_to_option_value() {
    check_options(Is::equal_to(Some(4)), |x| x == Some(4));
    check_options(Is::equal_to(None), |x| x.is_none());
}

#[test]
fn is_not_equal_to_option_value() {
    check_options(Is::not().equal_to(Some(4)), |x| x != Some(4));
}

#[test]
fn is_equal_to_string() {
    let e = Is::equal_to(String::from("hi"));
    assert!(e.eval(&String::from("hi")).pass);
    assert!(!e.eval(&String::from("Hi")).pass);
}

#[test]
fn is_equal_to_str_ref() {
    let e = Is::equal_to("hi");
    assert!(e.eval(&"hi").pass);
    assert!(!e.eval(&"bye").pass);
}

#[test]
fn is_equal_to_vec() {
    let e = Is::equal_to(vec![1, 2]);
    assert!(e.eval(&vec![1, 2]).pass);
    assert!(!e.eval(&vec![1, 2, 3]).pass);
}

#[test]
fn is_equal_to_float_nan_fails() {
    assert!(!Is::equal_to(f64::NAN).eval(&f64::NAN).pass);
    assert!(Is::not().equal_to(f64::NAN).eval(&f64::NAN).pass);
}

#[test]
fn is_equal_to_custom_struct() {
    #[derive(Debug, PartialEq)]
    struct User {
        id: u32,
    }
    assert!(Is::equal_to(User { id: 1 }).eval(&User { id: 1 }).pass);
    assert!(!Is::equal_to(User { id: 1 }).eval(&User { id: 2 }).pass);
}

// ---------------------------------------------------------------- or chains

#[test]
fn or_two() {
    check_ints(Is::equal_to(1).or().equal_to(2), |x| x == 1 || x == 2);
}

#[test]
fn or_three() {
    check_ints(Is::equal_to(1).or().equal_to(2).or().equal_to(3), |x| {
        (1..=3).contains(&x)
    });
}

#[test]
fn or_many() {
    let e = Is::equal_to(-7)
        .or()
        .equal_to(-3)
        .or()
        .equal_to(0)
        .or()
        .equal_to(2)
        .or()
        .equal_to(5)
        .or()
        .equal_to(9);
    check_ints(e, |x| [-7, -3, 0, 2, 5, 9].contains(&x));
}

#[test]
fn or_not() {
    check_ints(Is::equal_to(1).or().not().equal_to(1), |_| true);
    check_ints(Is::equal_to(1).or().not().equal_to(2), |x| x != 2);
}

#[test]
fn not_or_not() {
    check_ints(Is::not().equal_to(1).or().not().equal_to(2), |x| {
        x != 1 || x != 2
    });
}

#[test]
fn or_same_value_twice() {
    check_ints(Is::equal_to(4).or().equal_to(4), |x| x == 4);
}

// ---------------------------------------------------------------- and chains

#[test]
fn and_same_value() {
    check_ints(Is::equal_to(1).and().equal_to(1), |x| x == 1);
}

#[test]
fn and_different_values_never_passes() {
    check_ints(Is::equal_to(1).and().equal_to(2), |_| false);
}

#[test]
fn and_contradiction_never_passes() {
    check_ints(Is::equal_to(1).and().not().equal_to(1), |_| false);
}

#[test]
fn and_not_not() {
    check_ints(Is::not().equal_to(1).and().not().equal_to(2), |x| {
        x != 1 && x != 2
    });
}

#[test]
fn and_many_excludes_range() {
    let e = Is::not()
        .equal_to(1)
        .and()
        .not()
        .equal_to(2)
        .and()
        .not()
        .equal_to(3)
        .and()
        .not()
        .equal_to(4)
        .and()
        .not()
        .equal_to(5);
    check_ints(e, |x| !(1..=5).contains(&x));
}

// ---------------------------------------------------------------- precedence

#[test]
fn and_binds_tighter_than_or_distinguishing_case() {
    // a || (b && c)  vs  (a || b) && c  differ when a is true and c is false.
    let e = Is::equal_to(3).or().equal_to(5).and().not().equal_to(3);
    let r = e.eval(&3);
    assert!(
        r.pass,
        "3 should pass as `3 || (5 && !3)`; got {:?}",
        r.message
    );
    check_ints(
        Is::equal_to(3).or().equal_to(5).and().not().equal_to(3),
        |x| x == 3 || (x == 5 && x != 3),
    );
}

#[test]
fn and_then_or() {
    // (!1 && !2) || 1   ==   x != 2
    check_ints(
        Is::not()
            .equal_to(1)
            .and()
            .not()
            .equal_to(2)
            .or()
            .equal_to(1),
        |x| (x != 1 && x != 2) || x == 1,
    );
}

#[test]
fn or_and_or() {
    // 1 || (!2 && !3) || 3   ==   x != 2
    check_ints(
        Is::equal_to(1)
            .or()
            .not()
            .equal_to(2)
            .and()
            .not()
            .equal_to(3)
            .or()
            .equal_to(3),
        |x| x == 1 || (x != 2 && x != 3) || x == 3,
    );
}

#[test]
fn and_or_and() {
    // (!0 && !1) || (!2 && !3)
    check_ints(
        Is::not()
            .equal_to(0)
            .and()
            .not()
            .equal_to(1)
            .or()
            .not()
            .equal_to(2)
            .and()
            .not()
            .equal_to(3),
        |x| (x != 0 && x != 1) || (x != 2 && x != 3),
    );
}

#[test]
fn and_groups_reset_after_or() {
    // (0 && !1) || (2 && !3) -> 0 or 2
    check_ints(
        Is::equal_to(0)
            .and()
            .not()
            .equal_to(1)
            .or()
            .equal_to(2)
            .and()
            .not()
            .equal_to(3),
        |x| x == 0 || x == 2,
    );
}

#[test]
fn or_followed_by_contradictory_and_group() {
    // 1 || (2 && 3) || 4
    check_ints(
        Is::equal_to(1)
            .or()
            .equal_to(2)
            .and()
            .equal_to(3)
            .or()
            .equal_to(4),
        |x| x == 1 || x == 4,
    );
}

#[test]
fn long_alternating_chain() {
    // (!0 && !1 && !2) || 1 || (!5 && 6) || (-1 && !-1)
    let e = Is::not()
        .equal_to(0)
        .and()
        .not()
        .equal_to(1)
        .and()
        .not()
        .equal_to(2)
        .or()
        .equal_to(1)
        .or()
        .not()
        .equal_to(5)
        .and()
        .equal_to(6)
        .or()
        .equal_to(-1)
        .and()
        .not()
        .equal_to(-1);
    check_ints(e, |x| {
        (x != 0 && x != 1 && x != 2) || x == 1 || (x != 5 && x == 6) || (x == -1 && x != -1)
    });
}

#[test]
fn first_and_group_starting_with_not_then_or() {
    check_ints(Is::not().equal_to(0).or().equal_to(0), |_| true);
}

// ---------------------------------------------------------------- options mixing none + equal_to

#[test]
fn none_or_equal() {
    check_options(Is::none().or().equal_to(Some(5)), |x| {
        x.is_none() || x == Some(5)
    });
}

#[test]
fn equal_or_none() {
    check_options(Is::equal_to(Some(1)).or().none(), |x| {
        x == Some(1) || x.is_none()
    });
}

#[test]
fn equal_or_not_none() {
    check_options(Is::equal_to(Some(1)).or().not().none(), |x| x.is_some());
}

#[test]
fn not_none_or_none_always_passes() {
    check_options(Is::not().none().or().none(), |_| true);
}

#[test]
fn none_or_not_none_always_passes() {
    check_options(Is::none().or().not().none(), |_| true);
}

#[test]
fn none_and_not_none_never_passes() {
    check_options(Is::none().and().not().none(), |_| false);
}

#[test]
fn none_and_none() {
    check_options(Is::none().and().none(), |x| x.is_none());
}

#[test]
fn equal_and_none_never_passes() {
    check_options(Is::equal_to(Some(1)).and().none(), |_| false);
}

#[test]
fn equal_and_not_none() {
    check_options(Is::equal_to(Some(1)).and().not().none(), |x| x == Some(1));
}

#[test]
fn not_none_and_not_equal() {
    check_options(Is::not().none().and().not().equal_to(Some(5)), |x| {
        x.is_some() && x != Some(5)
    });
}

#[test]
fn not_none_and_equal() {
    check_options(Is::not().none().and().equal_to(Some(2)), |x| x == Some(2));
}

#[test]
fn none_or_not_equal() {
    check_options(Is::none().or().not().equal_to(Some(3)), |x| x != Some(3));
}

#[test]
fn option_complex_chain() {
    // None || (!None && !Some(1) && !Some(2)) || Some(2)
    let e = Is::none()
        .or()
        .not()
        .none()
        .and()
        .not()
        .equal_to(Some(1))
        .and()
        .not()
        .equal_to(Some(2))
        .or()
        .equal_to(Some(2));
    check_options(e, |x| x != Some(1));
}

#[test]
fn option_of_string() {
    let e = Is::none().or().equal_to(Some(String::from("x")));
    assert!(e.eval(&None).pass);
    assert!(e.eval(&Some("x".to_string())).pass);
    assert!(!e.eval(&Some("y".to_string())).pass);
}

// ---------------------------------------------------------------- failure messages

#[test]
fn message_single_equal_has_no_seed_noise() {
    let r = Is::equal_to(5).eval(&3);
    assert!(!r.pass);
    assert_eq!(r.message, "Expected 5, got 3");
}

#[test]
fn message_single_none() {
    let r = Is::none().eval(&Some(1));
    assert_eq!(r.message, "Expected None, got Some(_)");
}

#[test]
fn message_not_none() {
    let r = Is::not().none().eval(&None::<i32>);
    assert_eq!(r.message, "Expected not None, but it was");
}

#[test]
fn message_not_equal() {
    let r = Is::not().equal_to(1).eval(&1);
    assert_eq!(r.message, "Expected not equal to 1, but it was");
}

#[test]
fn message_or_chain_is_flat() {
    let r = Is::equal_to(1).or().equal_to(2).eval(&3);
    assert_eq!(
        r.message,
        either(&["Expected 1, got 3", "Expected 2, got 3"])
    );
}

#[test]
fn message_three_way_or_chain_is_flat() {
    let r = Is::equal_to(1).or().equal_to(2).or().equal_to(3).eval(&0);
    assert_eq!(
        r.message,
        either(&[
            "Expected 1, got 0",
            "Expected 2, got 0",
            "Expected 3, got 0"
        ])
    );
}

#[test]
fn message_long_or_chain_is_flat() {
    let r = Is::equal_to(1)
        .or()
        .equal_to(2)
        .or()
        .equal_to(3)
        .or()
        .equal_to(4)
        .or()
        .equal_to(5)
        .eval(&0);
    assert_eq!(
        r.message,
        either(&[
            "Expected 1, got 0",
            "Expected 2, got 0",
            "Expected 3, got 0",
            "Expected 4, got 0",
            "Expected 5, got 0",
        ])
    );
}

#[test]
fn message_and_reports_first_failing_member() {
    let r = Is::equal_to(1).and().equal_to(2).eval(&1);
    assert_eq!(r.message, "Expected 2, got 1");

    let r = Is::equal_to(1).and().equal_to(2).eval(&5);
    assert_eq!(r.message, "Expected 1, got 5");
}

#[test]
fn message_and_with_not_first() {
    let r = Is::not().equal_to(1).and().equal_to(2).eval(&1);
    assert_eq!(r.message, "Expected not equal to 1, but it was");
}

#[test]
fn message_and_then_or() {
    // (1 && 2) || 3, on 1: and-group fails at 2.
    let r = Is::equal_to(1).and().equal_to(2).or().equal_to(3).eval(&1);
    assert_eq!(
        r.message,
        either(&["Expected 2, got 1", "Expected 3, got 1"])
    );
}

#[test]
fn message_or_then_and() {
    // 1 || (!2 && 3), on 2: second group fails at !2.
    let r = Is::equal_to(1)
        .or()
        .not()
        .equal_to(2)
        .and()
        .equal_to(3)
        .eval(&2);
    assert_eq!(
        r.message,
        either(&["Expected 1, got 2", "Expected not equal to 2, but it was"])
    );
}

#[test]
fn message_options() {
    let r = Is::none().or().equal_to(Some(5)).eval(&Some(6));
    assert_eq!(
        r.message,
        either(&[
            "Expected None, got Some(_)",
            "Expected Some(5), got Some(6)"
        ])
    );
}

#[test]
fn message_string_value() {
    let r = Is::equal_to("abc").eval(&"abd");
    assert_eq!(r.message, "Expected \"abc\", got \"abd\"");
}

#[test]
fn message_matches_nested_query_flattens() {
    let r = Is::matches(Is::equal_to(1).or().equal_to(2))
        .or()
        .equal_to(3)
        .eval(&0);
    assert_eq!(
        r.message,
        either(&[
            "Expected 1, got 0",
            "Expected 2, got 0",
            "Expected 3, got 0"
        ])
    );
}

#[test]
fn message_not_of_nested_query_describes_it() {
    let r = Is::not().matches(Is::equal_to(1).or().equal_to(2)).eval(&2);
    assert_eq!(
        r.message,
        "Expected not (equal to 1 or equal to 2), but it was"
    );
}

#[test]
fn passing_queries_have_empty_messages() {
    assert_eq!(Is::equal_to(1).eval(&1).message, "");
    assert_eq!(Is::none().eval(&None::<i32>).message, "");
    assert_eq!(Is::not().none().eval(&Some(1)).message, "");
    assert_eq!(Is::equal_to(1).or().equal_to(2).eval(&2).message, "");
    assert_eq!(
        Is::not()
            .equal_to(1)
            .and()
            .not()
            .equal_to(2)
            .eval(&3)
            .message,
        ""
    );
}

// ---------------------------------------------------------------- reuse & misc

#[test]
fn query_is_reusable_across_many_evaluations() {
    let e = Is::equal_to(1).or().equal_to(2);
    for _ in 0..100 {
        assert!(e.eval(&1).pass);
        assert!(e.eval(&2).pass);
        assert!(!e.eval(&3).pass);
    }
}

#[test]
fn query_usable_as_dyn_expr() {
    let e: Box<dyn Expr<i32>> = Box::new(Is::equal_to(1).or().equal_to(2));
    assert!(e.eval(&2).pass);
    assert!(!e.eval(&3).pass);
}

#[test]
fn heterogeneous_queries_in_a_vec_of_dyn() {
    let exprs: Vec<Box<dyn Expr<i32>>> = vec![
        Box::new(Is::equal_to(1)),
        Box::new(Is::not().equal_to(1)),
        Box::new(Is::equal_to(1).or().equal_to(2)),
        Box::new(Is::not().equal_to(1).and().not().equal_to(2)),
    ];
    let on_1: Vec<bool> = exprs.iter().map(|e| e.eval(&1).pass).collect();
    let on_3: Vec<bool> = exprs.iter().map(|e| e.eval(&3).pass).collect();
    assert_eq!(on_1, [true, false, true, false]);
    assert_eq!(on_3, [false, true, false, true]);
}

#[test]
fn query_works_with_generic_helper() {
    fn passes<T, E: Expr<T>>(e: &E, v: &T) -> bool {
        e.eval(v).pass
    }
    assert!(passes(&Is::equal_to(1u8), &1u8));
    assert!(passes(&Is::none(), &None::<char>));
}

#[test]
fn query_works_with_non_copy_values() {
    let e = Is::equal_to(vec![String::from("a")])
        .or()
        .equal_to(vec![String::from("b"), String::from("c")]);
    assert!(e.eval(&vec!["a".into()]).pass);
    assert!(e.eval(&vec!["b".into(), "c".into()]).pass);
    assert!(!e.eval(&vec![]).pass);
}

#[test]
fn query_drops_owned_values() {
    use std::rc::Rc;
    let tracker = Rc::new(());
    {
        let e = Is::equal_to(tracker.clone()).or().equal_to(tracker.clone());
        assert_eq!(Rc::strong_count(&tracker), 3);
        assert!(e.eval(&tracker.clone()).pass);
    }
    assert_eq!(Rc::strong_count(&tracker), 1);
}

#[test]
fn intermediate_builder_types_are_nameable() {
    // Pending/Negated types are public, so builder fragments can be stored and finished later.
    let pending_or: PendingOr<_, _> = Is::equal_to(1).or();
    let pending_and: PendingAnd<_, _> = Is::equal_to(1).and();
    let negated: Negated<runit::Start> = Is::not();

    check_ints(pending_or.equal_to(2), |x| x == 1 || x == 2);
    check_ints(pending_and.not().equal_to(2), |x| x == 1);
    check_ints(negated.equal_to(0), |x| x != 0);
}

#[test]
fn negated_inner_field_is_public() {
    let n = Is::not();
    let runit::Start = n.inner;
}

#[test]
fn double_negation_cancels_at_start() {
    let _: runit::Start = Is::not().not();
    check_ints(Is::not().not().equal_to(3), |x| x == 3);
    check_options(Is::not().not().none(), |x| x.is_none());
}

#[test]
fn double_negation_cancels_after_or() {
    let _: PendingOr<_, _> = Is::equal_to(1).or().not().not();
    check_ints(Is::equal_to(1).or().not().not().equal_to(2), |x| {
        x == 1 || x == 2
    });
}

#[test]
fn double_negation_cancels_after_and() {
    let _: PendingAnd<_, _> = Is::equal_to(1).and().not().not();
    check_ints(Is::not().equal_to(1).and().not().not().equal_to(2), |x| {
        x == 2
    });
}

#[test]
fn triple_negation_equals_single() {
    check_ints(Is::not().not().not().equal_to(3), |x| x != 3);
    check_ints(Is::equal_to(0).or().not().not().not().equal_to(3), |x| {
        x != 3
    });
    check_options(Is::none().and().not().not().not().none(), |_| false);
}

#[test]
fn quadruple_negation_equals_none() {
    check_ints(Is::not().not().not().not().equal_to(3), |x| x == 3);
}

#[test]
fn double_negation_message_has_no_not() {
    assert_eq!(
        Is::not().not().equal_to(1).eval(&2).message,
        "Expected 1, got 2"
    );
}

// ---------------------------------------------------------------- matches (custom exprs)

struct GreaterThan(i32);

impl Expr<i32> for GreaterThan {
    fn eval(&self, actual: &i32) -> runit::ExprResult {
        if *actual > self.0 {
            runit::ExprResult::pass()
        } else {
            runit::ExprResult::fail(format!("Expected > {}, got {}", self.0, actual))
        }
    }

    fn describe(&self) -> String {
        format!("greater than {}", self.0)
    }
}

struct StartsWith(&'static str);

impl Expr<str> for StartsWith {
    fn eval(&self, actual: &str) -> runit::ExprResult {
        if actual.starts_with(self.0) {
            runit::ExprResult::pass()
        } else {
            runit::ExprResult::fail(format!("Expected {actual:?} to start with {:?}", self.0))
        }
    }

    fn describe(&self) -> String {
        format!("starting with {:?}", self.0)
    }
}

#[test]
fn matches_at_start() {
    check_ints(Is::matches(GreaterThan(3)), |x| x > 3);
}

#[test]
fn not_matches_at_start() {
    check_ints(Is::not().matches(GreaterThan(3)), |x| x <= 3);
}

#[test]
fn matches_after_or() {
    check_ints(Is::equal_to(-5).or().matches(GreaterThan(3)), |x| {
        x == -5 || x > 3
    });
}

#[test]
fn matches_after_and() {
    check_ints(
        Is::matches(GreaterThan(0))
            .and()
            .not()
            .matches(GreaterThan(5)),
        |x| x > 0 && x <= 5,
    );
}

#[test]
fn not_matches_after_or() {
    check_ints(Is::equal_to(9).or().not().matches(GreaterThan(-3)), |x| {
        x == 9 || x <= -3
    });
}

#[test]
fn matches_mixed_with_builtins_and_precedence() {
    // >8 || (>0 && !3 && !4)
    let e = Is::matches(GreaterThan(8))
        .or()
        .matches(GreaterThan(0))
        .and()
        .not()
        .equal_to(3)
        .and()
        .not()
        .equal_to(4);
    check_ints(e, |x| x > 8 || (x > 0 && x != 3 && x != 4));
}

#[test]
fn matches_nested_query() {
    let e = Is::matches(Is::equal_to(1).or().equal_to(2))
        .and()
        .not()
        .equal_to(2);
    check_ints(e, |x| x == 1);
}

#[test]
fn matches_on_unsized_str() {
    let e = Is::matches(StartsWith("http://"))
        .or()
        .matches(StartsWith("https://"));
    assert!(e.eval("https://x").pass);
    assert!(!e.eval("ftp://x").pass);
    assert_eq!(
        e.eval("ftp").message,
        either(&[
            "Expected \"ftp\" to start with \"http://\"",
            "Expected \"ftp\" to start with \"https://\"",
        ])
    );
    assert_eq!(
        e.describe(),
        "starting with \"http://\" or starting with \"https://\""
    );
    runit::Assert::that("http://x", e);
}

#[test]
fn matches_message_uses_custom_messages() {
    assert_eq!(
        Is::matches(GreaterThan(3)).eval(&1).message,
        "Expected > 3, got 1"
    );
    assert_eq!(
        Is::not().matches(GreaterThan(3)).eval(&5).message,
        "Expected not greater than 3, but it was"
    );
}

#[test]
fn matches_accepts_references_and_boxes() {
    let gt = GreaterThan(2);
    check_ints(Is::matches(&gt), |x| x > 2);
    let boxed: Box<dyn Expr<i32>> = Box::new(GreaterThan(2));
    check_ints(Is::not().matches(boxed), |x| x <= 2);
    // `gt` is still usable.
    assert!(gt.eval(&3).pass);
}

#[test]
fn matches_with_closure_adapter() {
    struct Pred<F>(F, &'static str);
    impl<F: Fn(&i32) -> bool> Expr<i32> for Pred<F> {
        fn eval(&self, actual: &i32) -> runit::ExprResult {
            if (self.0)(actual) {
                runit::ExprResult::pass()
            } else {
                runit::ExprResult::fail(format!("Expected {}, got {actual}", self.1))
            }
        }
    }
    let e = Is::matches(Pred(|x: &i32| x % 2 == 0, "even"))
        .or()
        .equal_to(7);
    check_ints(e, |x| x % 2 == 0 || x == 7);
}

// ---------------------------------------------------------------- Step trait

#[test]
fn step_trait_allows_generic_builder_helpers() {
    use runit::Step;
    fn not_zero<S: Step>(s: S) -> S::Out<runit::Not<runit::Equal<i32>>> {
        s.then(runit::Not {
            inner: runit::Equal { value: 0 },
        })
    }
    check_ints(not_zero(runit::Start), |x| x != 0);
    check_ints(not_zero(Is::equal_to(0).or()), |_| true);
    check_ints(not_zero(Is::equal_to(1).and()), |x| x == 1);
    check_ints(not_zero(Is::not()), |x| x == 0);
}
