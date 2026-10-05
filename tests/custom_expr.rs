//! Tests for user-defined `Expr` implementations and the `describe` default.

mod common;

use common::panic_message;
use runit::{AlwaysFalse, And, Assert, Equal, Expr, ExprResult, Is, IsNone, Not, Or};

struct GreaterThan(i32);

impl Expr<i32> for GreaterThan {
    fn eval(&self, actual: &i32) -> ExprResult {
        if *actual > self.0 {
            ExprResult::pass()
        } else {
            ExprResult::fail(format!("Expected > {}, got {}", self.0, actual))
        }
    }
}

struct LessThan(i32);

impl Expr<i32> for LessThan {
    fn eval(&self, actual: &i32) -> ExprResult {
        if *actual < self.0 {
            ExprResult::pass()
        } else {
            ExprResult::fail(format!("Expected < {}, got {}", self.0, actual))
        }
    }

    fn describe(&self) -> String {
        format!("less than {}", self.0)
    }
}

/// Generic over any `PartialOrd` type, including unsized ones.
struct AtMost<T>(T);

impl<T: PartialOrd + std::fmt::Debug> Expr<T> for AtMost<T> {
    fn eval(&self, actual: &T) -> ExprResult {
        if *actual <= self.0 {
            ExprResult::pass()
        } else {
            ExprResult::fail(format!("Expected <= {:?}, got {:?}", self.0, actual))
        }
    }
}

struct StartsWith(&'static str);

impl Expr<str> for StartsWith {
    fn eval(&self, actual: &str) -> ExprResult {
        if actual.starts_with(self.0) {
            ExprResult::pass()
        } else {
            ExprResult::fail(format!("Expected {actual:?} to start with {:?}", self.0))
        }
    }
}

// String delegates to str so the same expr works for both.
impl Expr<String> for StartsWith {
    fn eval(&self, actual: &String) -> ExprResult {
        <Self as Expr<str>>::eval(self, actual)
    }
}

struct Contains<T>(T);

impl<T: PartialEq + std::fmt::Debug> Expr<[T]> for Contains<T> {
    fn eval(&self, actual: &[T]) -> ExprResult {
        if actual.contains(&self.0) {
            ExprResult::pass()
        } else {
            ExprResult::fail(format!("Expected {actual:?} to contain {:?}", self.0))
        }
    }
}

/// An expr that borrows its expected value.
struct EqualsRef<'a, T>(&'a T);

impl<T: PartialEq> Expr<T> for EqualsRef<'_, T> {
    fn eval(&self, actual: &T) -> ExprResult {
        if self.0 == actual {
            ExprResult::pass()
        } else {
            ExprResult::fail("not equal")
        }
    }
}

// ---------------------------------------------------------------- basic custom exprs

#[test]
fn custom_expr_pass_and_fail() {
    assert!(GreaterThan(5).eval(&6).pass);
    let r = GreaterThan(5).eval(&5);
    assert!(!r.pass);
    assert_eq!(r.message, "Expected > 5, got 5");
}

#[test]
fn custom_expr_with_assert() {
    Assert::that(&10, GreaterThan(5));
    assert_eq!(
        panic_message(|| Assert::that(&1, GreaterThan(5))),
        "Expected > 5, got 1"
    );
}

#[test]
fn generic_custom_expr_over_several_types() {
    assert!(AtMost(3).eval(&3).pass);
    assert!(AtMost(1.5).eval(&1.0).pass);
    assert!(AtMost('m').eval(&'a').pass);
    assert!(!AtMost(String::from("b")).eval(&String::from("c")).pass);
}

#[test]
fn custom_expr_on_str_and_string() {
    assert!(<_ as Expr<str>>::eval(&StartsWith("ab"), "abc").pass);
    assert!(StartsWith("ab").eval(&String::from("abc")).pass);
    assert_eq!(
        <_ as Expr<str>>::eval(&StartsWith("x"), "abc").message,
        "Expected \"abc\" to start with \"x\""
    );
}

#[test]
fn custom_expr_on_slices() {
    assert!(Contains(2).eval(&[1, 2, 3][..]).pass);
    assert!(!Contains(9).eval(&[1, 2, 3][..]).pass);
    assert!(!Contains(1).eval(&[][..]).pass);
    let v = vec!["a", "b"];
    Assert::that(v.as_slice(), Contains("b"));
}

#[test]
fn custom_expr_borrowing_expected_value() {
    let expected = vec![1, 2];
    assert!(EqualsRef(&expected).eval(&vec![1, 2]).pass);
    assert!(!EqualsRef(&expected).eval(&vec![]).pass);
}

#[test]
fn custom_expr_with_interior_state() {
    use std::cell::Cell;
    struct PassesFirstNTimes(Cell<u32>);
    impl Expr<()> for PassesFirstNTimes {
        fn eval(&self, _: &()) -> ExprResult {
            let n = self.0.get();
            if n == 0 {
                ExprResult::fail("exhausted")
            } else {
                self.0.set(n - 1);
                ExprResult::pass()
            }
        }
    }
    let e = PassesFirstNTimes(Cell::new(2));
    assert!(e.eval(&()).pass);
    assert!(e.eval(&()).pass);
    assert!(!e.eval(&()).pass);
}

// ---------------------------------------------------------------- composing with built-in combinators

#[test]
fn custom_exprs_compose_into_range_check() {
    let between = And {
        left: GreaterThan(0),
        right: LessThan(5),
    };
    for x in common::INTS {
        assert_eq!(between.eval(&x).pass, x > 0 && x < 5, "x = {x}");
    }
    assert_eq!(between.eval(&0).message, "Expected > 0, got 0");
    assert_eq!(between.eval(&5).message, "Expected < 5, got 5");
}

#[test]
fn custom_exprs_compose_with_or_and_not() {
    // outside [-2, 2]
    let outside = Or {
        left: LessThan(-2),
        right: GreaterThan(2),
    };
    let inside = Not {
        inner: Or {
            left: LessThan(-2),
            right: GreaterThan(2),
        },
    };
    for x in common::INTS {
        assert_eq!(outside.eval(&x).pass, !(-2..=2).contains(&x));
        assert_eq!(inside.eval(&x).pass, (-2..=2).contains(&x));
    }
}

#[test]
fn custom_exprs_mix_with_builtin_primitives() {
    let e = Or {
        left: Equal { value: 0 },
        right: And {
            left: GreaterThan(5),
            right: Not {
                inner: Equal { value: 7 },
            },
        },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x == 0 || (x > 5 && x != 7));
    }
}

#[test]
fn custom_exprs_compose_with_builder_query() {
    // Builder output is an ordinary Expr, so it nests inside the raw combinators.
    let e = And {
        left: Is::not().equal_to(3).and().not().equal_to(4),
        right: GreaterThan(0),
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x > 0 && x != 3 && x != 4);
    }
}

#[test]
fn custom_unsized_exprs_compose() {
    let e = Or {
        left: StartsWith("http://"),
        right: StartsWith("https://"),
    };
    for (s, ok) in [
        ("http://a", true),
        ("https://a", true),
        ("ftp://a", false),
        ("", false),
    ] {
        assert_eq!(<_ as Expr<str>>::eval(&e, s).pass, ok, "{s:?}");
    }
}

#[test]
fn custom_slice_exprs_compose() {
    let e = And {
        left: Contains(1),
        right: Not { inner: Contains(0) },
    };
    assert!(e.eval(&[1, 2][..]).pass);
    assert!(!e.eval(&[1, 0][..]).pass);
    assert!(!e.eval(&[2][..]).pass);
}

// ---------------------------------------------------------------- dyn usage

#[test]
fn expr_trait_is_dyn_compatible() {
    let exprs: Vec<Box<dyn Expr<i32>>> = vec![
        Box::new(GreaterThan(0)),
        Box::new(LessThan(0)),
        Box::new(AtMost(0)),
        Box::new(Is::equal_to(0)),
    ];
    let on_zero: Vec<bool> = exprs.iter().map(|e| e.eval(&0).pass).collect();
    assert_eq!(on_zero, [false, false, true, true]);
}

#[test]
fn dyn_expr_over_unsized_type() {
    let exprs: Vec<Box<dyn Expr<str>>> = vec![Box::new(StartsWith("a")), Box::new(AlwaysFalse)];
    let res: Vec<bool> = exprs.iter().map(|e| e.eval("abc").pass).collect();
    assert_eq!(res, [true, false]);
}

#[test]
fn dyn_expr_describe_dispatches_dynamically() {
    let e: Box<dyn Expr<i32>> = Box::new(LessThan(3));
    assert_eq!(e.describe(), "less than 3");
}

// ---------------------------------------------------------------- describe

#[test]
fn describe_default_is_type_name() {
    let d = <GreaterThan as Expr<i32>>::describe(&GreaterThan(1));
    assert!(d.ends_with("GreaterThan"), "{d}");
    assert!(d.contains("custom_expr"), "{d}");
}

#[test]
fn describe_can_be_overridden() {
    assert_eq!(LessThan(9).describe(), "less than 9");
}

#[test]
fn describe_for_generic_custom_expr_includes_type_param() {
    let d = <AtMost<u8> as Expr<u8>>::describe(&AtMost(1u8));
    assert!(d.contains("AtMost<u8>"), "{d}");
}

#[test]
fn describe_builtins() {
    assert_eq!(<IsNone as Expr<Option<i32>>>::describe(&IsNone), "None");
    assert_eq!(
        <AlwaysFalse as Expr<i32>>::describe(&AlwaysFalse),
        "always false"
    );
    assert_eq!(Equal { value: 1 }.describe(), "equal to 1");
    assert_eq!(Equal { value: "s" }.describe(), "equal to \"s\"");
    assert_eq!(Equal { value: Some(1) }.describe(), "equal to Some(1)");
    assert_eq!(
        <Not<IsNone> as Expr<Option<i32>>>::describe(&Not { inner: IsNone }),
        "not None"
    );
}

#[test]
fn describe_combinators_include_children() {
    let d = <_ as Expr<()>>::describe(&And {
        left: AlwaysFalse,
        right: Not { inner: AlwaysFalse },
    });
    assert_eq!(d, "always false and not always false");

    let d = <_ as Expr<Option<i32>>>::describe(&Or {
        left: IsNone,
        right: Equal { value: Some(1) },
    });
    assert_eq!(d, "None or equal to Some(1)");
}

#[test]
fn describe_parenthesizes_or_inside_and() {
    let e = And {
        left: Or {
            left: Equal { value: 1 },
            right: Equal { value: 2 },
        },
        right: Not {
            inner: Equal { value: 3 },
        },
    };
    assert_eq!(
        e.describe(),
        "(equal to 1 or equal to 2) and not equal to 3"
    );
}

#[test]
fn describe_does_not_parenthesize_and_inside_or() {
    let e = Or {
        left: And {
            left: Equal { value: 1 },
            right: Equal { value: 2 },
        },
        right: Equal { value: 3 },
    };
    assert_eq!(e.describe(), "equal to 1 and equal to 2 or equal to 3");
}

#[test]
fn describe_parenthesizes_compound_inside_not() {
    let e = Not {
        inner: Or {
            left: Equal { value: 1 },
            right: Equal { value: 2 },
        },
    };
    assert_eq!(e.describe(), "not (equal to 1 or equal to 2)");
    let e = Not {
        inner: And {
            left: Equal { value: 1 },
            right: Equal { value: 2 },
        },
    };
    assert_eq!(e.describe(), "not (equal to 1 and equal to 2)");
}

#[test]
fn describe_uses_child_overrides() {
    let e = Not { inner: LessThan(3) };
    assert_eq!(e.describe(), "not less than 3");
    assert_eq!(e.eval(&1).message, "Expected not less than 3, but it was");
}

#[test]
fn describe_falls_back_to_type_name_for_children_without_override() {
    let d = <_ as Expr<i32>>::describe(&Not {
        inner: GreaterThan(3),
    });
    assert!(d.starts_with("not "), "{d}");
    assert!(d.ends_with("GreaterThan"), "{d}");
}

#[test]
fn describe_builder_query() {
    assert_eq!(<_ as Expr<i32>>::describe(&Is::equal_to(1)), "equal to 1");
    assert_eq!(
        <_ as Expr<i32>>::describe(&Is::equal_to(1).or().not().equal_to(2)),
        "equal to 1 or not equal to 2"
    );
    assert_eq!(
        <_ as Expr<Option<i32>>>::describe(&Is::none().and().not().none()),
        "None and not None"
    );
    assert_eq!(
        <_ as Expr<i32>>::describe(
            &Is::equal_to(1)
                .or()
                .equal_to(2)
                .and()
                .not()
                .equal_to(3)
                .or()
                .matches(LessThan(0))
        ),
        "equal to 1 or equal to 2 and not equal to 3 or less than 0"
    );
}

#[test]
fn describe_forwards_through_pointers() {
    let e = LessThan(4);
    assert_eq!(<&LessThan as Expr<i32>>::describe(&&e), "less than 4");
    assert_eq!(Box::new(LessThan(4)).describe(), "less than 4");
    assert_eq!(std::rc::Rc::new(LessThan(4)).describe(), "less than 4");
    assert_eq!(std::sync::Arc::new(LessThan(4)).describe(), "less than 4");
}

#[test]
fn pointer_exprs_evaluate_like_their_target() {
    let e = GreaterThan(0);
    let r: &dyn Expr<i32> = &e;
    let b: Box<dyn Expr<i32>> = Box::new(GreaterThan(0));
    let rc: std::rc::Rc<dyn Expr<i32>> = std::rc::Rc::new(GreaterThan(0));
    let arc: std::sync::Arc<dyn Expr<i32>> = std::sync::Arc::new(GreaterThan(0));
    for x in common::INTS {
        let want = x > 0;
        assert_eq!(<&GreaterThan as Expr<i32>>::eval(&&e, &x).pass, want);
        assert_eq!(Expr::eval(&r, &x).pass, want);
        assert_eq!(Expr::eval(&b, &x).pass, want);
        assert_eq!(Expr::eval(&rc, &x).pass, want);
        assert_eq!(Expr::eval(&arc, &x).pass, want);
    }
}

#[test]
fn pointer_exprs_compose_in_combinators() {
    let gt = GreaterThan(0);
    let lt = LessThan(5);
    let between = And {
        left: &gt,
        right: &lt,
    };
    let outside = Not { inner: &between };
    for x in common::INTS {
        assert_eq!(between.eval(&x).pass, x > 0 && x < 5);
        assert_eq!(outside.eval(&x).pass, !(x > 0 && x < 5));
    }
}
