mod common;

use common::{Const, EvalExt, assert_no_panic, panic_message};
use runit::{AlwaysFalse, And, Assert, Equal, Expr, Is, IsNone, Not, Or};

#[test]
fn passing_assertion_does_not_panic() {
    Assert::that(&5, Is::equal_to(5));
}

#[test]
fn passing_assertion_on_none() {
    let v: Option<i32> = None;
    Assert::that(&v, Is::none());
}

#[test]
fn passing_assertion_on_some() {
    Assert::that(&Some(1), Is::not().none());
}

#[test]
#[should_panic(expected = "Expected 5, got 3")]
fn failing_equal_panics() {
    Assert::that(&3, Is::equal_to(5));
}

#[test]
#[should_panic(expected = "Expected None, got Some(_)")]
fn failing_none_panics() {
    Assert::that(&Some(1), Is::none());
}

#[test]
#[should_panic(expected = "Expected not None, but it was")]
fn failing_not_none_panics() {
    Assert::that(&None::<i32>, Is::not().none());
}

#[test]
#[should_panic(expected = "Expected not equal to 1, but it was")]
fn failing_not_equal_panics() {
    Assert::that(&1, Is::not().equal_to(1));
}

#[test]
fn panic_message_is_exactly_the_expr_message() {
    let msg = panic_message(|| Assert::that(&3, Is::equal_to(5)));
    assert_eq!(msg, "Expected 5, got 3");
}

#[test]
fn panic_message_for_or_chain_is_a_flat_list() {
    let msg = panic_message(|| Assert::that(&0, Is::equal_to(1).or().equal_to(2).or().equal_to(3)));
    assert_eq!(
        msg,
        "Expected either:\n  - Expected 1, got 0\n  - Expected 2, got 0\n  - Expected 3, got 0"
    );
}

#[test]
fn reusing_an_expr_by_reference() {
    let e = Is::equal_to(1).or().equal_to(2);
    Assert::that(&1, &e);
    Assert::that(&2, &e);
    assert_eq!(
        panic_message(|| Assert::that(&3, &e)),
        "Expected either:\n  - Expected 1, got 3\n  - Expected 2, got 3"
    );
}

#[cfg(feature = "alloc")]
#[test]
fn boxed_dyn_expr_works_with_assert() {
    let e: Box<dyn Expr<i32>> = Box::new(Is::not().equal_to(0));
    Assert::that(&1, &e);
    Assert::that(&2, e);
}

#[cfg(feature = "alloc")]
#[test]
fn rc_and_arc_exprs_work_with_assert() {
    let rc = std::rc::Rc::new(Is::equal_to(5));
    Assert::that(&5, rc.clone());
    Assert::that(&5, rc);
    let arc: std::sync::Arc<dyn Expr<str> + Send + Sync> = std::sync::Arc::new(AlwaysFalse);
    assert_eq!(
        panic_message(|| Assert::that("x", arc)),
        "Condition will always fail."
    );
}

#[test]
fn panic_message_for_raw_primitive() {
    assert_eq!(
        panic_message(|| Assert::that(&3, Equal { value: 5 })),
        "Expected 5, got 3"
    );
    assert_eq!(
        panic_message(|| Assert::that(&Some(1), IsNone)),
        "Expected None, got Some(_)"
    );
    assert_eq!(
        panic_message(|| Assert::that(&0, AlwaysFalse)),
        "Condition will always fail."
    );
}

#[test]
fn panic_message_preserves_custom_message() {
    let msg = panic_message(|| Assert::that(&(), Const(false, "custom failure\nwith newline")));
    assert_eq!(msg, "custom failure\nwith newline");
}

#[test]
fn panic_message_is_not_interpreted_as_format_string() {
    let msg = panic_message(|| Assert::that(&(), Const(false, "{} {:?} {{}}")));
    assert_eq!(msg, "{} {:?} {{}}");
}

#[test]
fn panic_payload_is_a_string() {
    let payload = std::panic::catch_unwind(|| Assert::that(&1, Equal { value: 2 })).unwrap_err();
    assert!(payload.downcast_ref::<String>().is_some());
}

#[test]
fn empty_failure_message_still_panics() {
    let msg = panic_message(|| Assert::that(&(), Const(false, "")));
    assert_eq!(msg, "");
}

#[test]
fn pass_with_message_does_not_panic() {
    Assert::that(&(), Const(true, "a note that should be ignored"));
}

#[test]
fn raw_primitives_work_with_assert() {
    Assert::that(&1, Equal { value: 1 });
    Assert::that(&None::<u8>, IsNone);
    Assert::that(&0, Not { inner: AlwaysFalse });
    Assert::that(
        &4,
        And {
            left: Not {
                inner: Equal { value: 1 },
            },
            right: Not {
                inner: Equal { value: 2 },
            },
        },
    );
    Assert::that(
        &2,
        Or {
            left: Equal { value: 1 },
            right: Equal { value: 2 },
        },
    );
}

#[test]
fn assert_with_unsized_str() {
    struct NonEmpty;
    impl Expr<str> for NonEmpty {
        fn check(&self, actual: &str) -> bool {
            !actual.is_empty()
        }

        fn explain(&self, _actual: &str, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Expected non-empty string")
        }
    }
    Assert::that("hello", NonEmpty);
    assert_eq!(
        panic_message(|| Assert::that("", NonEmpty)),
        "Expected non-empty string"
    );
}

#[test]
fn assert_with_unsized_slice() {
    struct Sorted;
    impl Expr<[i32]> for Sorted {
        fn check(&self, actual: &[i32]) -> bool {
            actual.windows(2).all(|w| w[0] <= w[1])
        }

        fn explain(&self, actual: &[i32], f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Expected sorted, got {actual:?}")
        }
    }
    Assert::that(&[1, 2, 3][..], Sorted);
    Assert::that(&[][..], Sorted);
    assert_eq!(
        panic_message(|| Assert::that(&[3, 1][..], Sorted)),
        "Expected sorted, got [3, 1]"
    );
}

#[test]
fn assert_with_unsized_combinators() {
    struct Len(usize);
    impl Expr<str> for Len {
        fn check(&self, actual: &str) -> bool {
            actual.len() == self.0
        }

        fn explain(&self, actual: &str, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Expected len {}, got {}", self.0, actual.len())
        }
    }
    Assert::that(
        "abc",
        Or {
            left: Len(1),
            right: Len(3),
        },
    );
    Assert::that("abc", Not { inner: Len(2) });
    assert_eq!(
        panic_message(|| Assert::that(
            "ab",
            And {
                left: Len(2),
                right: Len(3)
            }
        )),
        "Expected len 3, got 2"
    );
}

#[test]
fn assert_with_dyn_trait_actual() {
    struct DebugContains(&'static str);
    impl Expr<dyn std::fmt::Debug> for DebugContains {
        fn check(&self, actual: &dyn std::fmt::Debug) -> bool {
            format!("{actual:?}").contains(self.0)
        }

        fn explain(
            &self,
            actual: &dyn std::fmt::Debug,
            f: &mut std::fmt::Formatter<'_>,
        ) -> std::fmt::Result {
            write!(
                f,
                "{:?} does not contain {:?}",
                format!("{actual:?}"),
                self.0
            )
        }
    }
    let v: &dyn std::fmt::Debug = &vec![1, 2, 3];
    Assert::that(v, DebugContains("2, 3"));
    assert_no_panic(|| Assert::that(v, DebugContains("1")));
    assert_eq!(
        panic_message(|| Assert::that(v, DebugContains("9"))),
        "\"[1, 2, 3]\" does not contain \"9\""
    );
}

#[test]
fn assert_on_reference_to_reference() {
    let s = String::from("x");
    let r: &String = &s;
    Assert::that(&r, Is::equal_to(&s));
}

#[test]
fn many_assertions_in_sequence() {
    for x in common::INTS {
        Assert::that(&x, Is::equal_to(x));
        Assert::that(&x, Is::not().equal_to(x + 1));
        Assert::that(
            &x,
            Is::equal_to(x - 1).or().equal_to(x).or().equal_to(x + 1),
        );
    }
}

#[test]
fn assert_agrees_with_eval_for_every_value() {
    for x in common::INTS {
        let should_pass = x % 3 == 0 && x != 0;
        let build = || {
            Is::equal_to(-9)
                .or()
                .equal_to(-6)
                .or()
                .equal_to(-3)
                .or()
                .equal_to(3)
                .or()
                .equal_to(6)
                .or()
                .equal_to(9)
        };
        assert_eq!(build().eval(&x).pass, should_pass);
        let outcome = std::panic::catch_unwind(|| Assert::that(&x, build()));
        assert_eq!(outcome.is_ok(), should_pass, "x = {x}");
    }
}

#[test]
fn assert_consumes_expr_but_actual_is_borrowed() {
    let v = vec![1, 2, 3];
    Assert::that(&v, Is::equal_to(vec![1, 2, 3]));
    // `v` is still usable after the assertion.
    assert_eq!(v.len(), 3);
}
