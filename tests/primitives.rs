mod common;

use common::{Const, EvalExt, Probe};
use runit::{AlwaysFalse, And, Equal, Expr, IsNone, Not, Or};

const ALWAYS_FALSE_MSG: &str = "Condition will always fail.";

// ---------------------------------------------------------------- AlwaysFalse

#[test]
fn always_false_fails_for_ints() {
    for x in common::INTS {
        let r = AlwaysFalse.eval(&x);
        assert!(!r.pass);
        assert_eq!(r.message, ALWAYS_FALSE_MSG);
    }
}

#[test]
fn always_false_fails_for_unit() {
    assert!(!AlwaysFalse.eval(&()).pass);
}

#[test]
fn always_false_fails_for_options() {
    assert!(!AlwaysFalse.eval(&None::<i32>).pass);
    assert!(!AlwaysFalse.eval(&Some(1)).pass);
}

#[test]
fn always_false_fails_for_unsized_str() {
    let r = <AlwaysFalse as EvalExt<str>>::eval(&AlwaysFalse, "hello");
    assert!(!r.pass);
    assert_eq!(r.message, ALWAYS_FALSE_MSG);
}

#[test]
fn always_false_fails_for_unsized_slice() {
    let r = <AlwaysFalse as EvalExt<[u8]>>::eval(&AlwaysFalse, &[1, 2, 3][..]);
    assert!(!r.pass);
}

#[test]
fn always_false_fails_for_dyn_trait_object() {
    let d: &dyn std::fmt::Debug = &5;
    let r = <AlwaysFalse as EvalExt<dyn std::fmt::Debug>>::eval(&AlwaysFalse, d);
    assert!(!r.pass);
}

// ---------------------------------------------------------------- IsNone

#[test]
fn is_none_passes_for_none() {
    let r = IsNone.eval(&None::<i32>);
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn is_none_fails_for_some() {
    let r = IsNone.eval(&Some(0));
    assert!(!r.pass);
    assert_eq!(r.message, "Expected None, got Some(_)");
}

#[test]
fn is_none_works_with_non_debug_non_partial_eq_payload() {
    struct Opaque;
    assert!(IsNone.eval(&None::<Opaque>).pass);
    assert!(!IsNone.eval(&Some(Opaque)).pass);
}

#[test]
fn is_none_works_with_string_payload() {
    assert!(IsNone.eval(&None::<String>).pass);
    assert!(!IsNone.eval(&Some(String::new())).pass);
}

#[test]
fn is_none_works_with_reference_payload() {
    let s = String::from("x");
    assert!(IsNone.eval(&None::<&String>).pass);
    assert!(!IsNone.eval(&Some(&s)).pass);
}

#[test]
fn is_none_works_with_unit_payload() {
    assert!(IsNone.eval(&None::<()>).pass);
    assert!(!IsNone.eval(&Some(())).pass);
}

#[test]
fn is_none_on_nested_option_only_checks_outer_layer() {
    assert!(IsNone.eval(&None::<Option<i32>>).pass);
    assert!(!IsNone.eval(&Some(None::<i32>)).pass);
    assert!(!IsNone.eval(&Some(Some(1))).pass);
}

#[test]
fn is_none_is_reusable_across_evaluations() {
    let e = IsNone;
    for _ in 0..3 {
        assert!(e.eval(&None::<u8>).pass);
        assert!(!e.eval(&Some(1u8)).pass);
    }
}

// ---------------------------------------------------------------- Equal

#[test]
fn equal_passes_for_equal_ints() {
    let r = Equal { value: 5 }.eval(&5);
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn equal_fails_for_different_ints_with_debug_message() {
    let r = Equal { value: 5 }.eval(&3);
    assert!(!r.pass);
    assert_eq!(r.message, "Expected 5, got 3");
}

#[test]
fn equal_exhaustive_over_int_range() {
    for expected in common::INTS {
        let e = Equal { value: expected };
        for actual in common::INTS {
            assert_eq!(
                e.eval(&actual).pass,
                expected == actual,
                "{expected} vs {actual}"
            );
        }
    }
}

#[test]
fn equal_handles_integer_extremes() {
    assert!(Equal { value: i64::MIN }.eval(&i64::MIN).pass);
    assert!(Equal { value: u64::MAX }.eval(&u64::MAX).pass);
    assert_eq!(
        Equal { value: i8::MIN }.eval(&i8::MAX).message,
        "Expected -128, got 127"
    );
}

#[test]
fn equal_strings_use_debug_quoting() {
    let r = Equal {
        value: String::from("a"),
    }
    .eval(&String::from("b"));
    assert!(!r.pass);
    assert_eq!(r.message, "Expected \"a\", got \"b\"");
}

#[test]
fn equal_string_escapes_special_characters_in_message() {
    let r = Equal {
        value: String::from("a\nb"),
    }
    .eval(&String::from("\"q\""));
    assert_eq!(r.message, r#"Expected "a\nb", got "\"q\"""#);
}

#[test]
fn equal_str_refs() {
    assert!(Equal { value: "hi" }.eval(&"hi").pass);
    assert_eq!(
        Equal { value: "hi" }.eval(&"ho").message,
        "Expected \"hi\", got \"ho\""
    );
}

#[test]
fn equal_empty_strings() {
    assert!(Equal { value: "" }.eval(&"").pass);
    assert!(!Equal { value: "" }.eval(&" ").pass);
}

#[test]
fn equal_chars() {
    assert!(Equal { value: 'a' }.eval(&'a').pass);
    assert_eq!(
        Equal { value: 'a' }.eval(&'b').message,
        "Expected 'a', got 'b'"
    );
}

#[test]
fn equal_bools() {
    assert!(Equal { value: true }.eval(&true).pass);
    assert!(Equal { value: false }.eval(&false).pass);
    assert_eq!(
        Equal { value: true }.eval(&false).message,
        "Expected true, got false"
    );
}

#[test]
fn equal_unit() {
    assert!(Equal { value: () }.eval(&()).pass);
}

#[test]
fn equal_floats() {
    assert!(Equal { value: 1.5f64 }.eval(&1.5).pass);
    assert_eq!(
        Equal { value: 1.5f64 }.eval(&2.0).message,
        "Expected 1.5, got 2.0"
    );
}

#[test]
fn equal_float_signed_zero_is_equal() {
    assert!(Equal { value: 0.0f64 }.eval(&-0.0).pass);
}

#[test]
fn equal_float_nan_is_never_equal_to_itself() {
    let r = Equal { value: f64::NAN }.eval(&f64::NAN);
    assert!(!r.pass);
    assert_eq!(r.message, "Expected NaN, got NaN");
}

#[test]
fn equal_float_infinities() {
    assert!(
        Equal {
            value: f64::INFINITY
        }
        .eval(&f64::INFINITY)
        .pass
    );
    assert!(
        !Equal {
            value: f64::INFINITY
        }
        .eval(&f64::NEG_INFINITY)
        .pass
    );
}

#[test]
fn equal_vecs() {
    assert!(
        Equal {
            value: vec![1, 2, 3]
        }
        .eval(&vec![1, 2, 3])
        .pass
    );
    assert_eq!(
        Equal { value: vec![1, 2] }.eval(&vec![2, 1]).message,
        "Expected [1, 2], got [2, 1]"
    );
}

#[test]
fn equal_empty_vecs() {
    assert!(
        Equal {
            value: Vec::<i32>::new()
        }
        .eval(&vec![])
        .pass
    );
}

#[test]
fn equal_arrays() {
    assert!(Equal { value: [1u8; 4] }.eval(&[1, 1, 1, 1]).pass);
    assert!(!Equal { value: [1u8; 4] }.eval(&[1, 1, 1, 0]).pass);
}

#[test]
fn equal_tuples() {
    assert!(Equal { value: (1, "a") }.eval(&(1, "a")).pass);
    assert_eq!(
        Equal { value: (1, "a") }.eval(&(2, "a")).message,
        "Expected (1, \"a\"), got (2, \"a\")"
    );
}

#[test]
fn equal_options() {
    assert!(Equal { value: Some(1) }.eval(&Some(1)).pass);
    assert!(Equal { value: None::<i32> }.eval(&None).pass);
    assert_eq!(
        Equal { value: Some(1) }.eval(&None).message,
        "Expected Some(1), got None"
    );
    assert_eq!(
        Equal { value: None }.eval(&Some(1)).message,
        "Expected None, got Some(1)"
    );
}

#[test]
fn equal_results() {
    let ok: Result<i32, String> = Ok(1);
    let err: Result<i32, String> = Err("boom".into());
    assert!(Equal { value: ok.clone() }.eval(&ok).pass);
    assert_eq!(
        Equal { value: ok }.eval(&err).message,
        "Expected Ok(1), got Err(\"boom\")"
    );
}

#[test]
fn equal_custom_derived_struct() {
    #[derive(Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }
    assert!(
        Equal {
            value: Point { x: 1, y: 2 }
        }
        .eval(&Point { x: 1, y: 2 })
        .pass
    );
    assert_eq!(
        Equal {
            value: Point { x: 1, y: 2 }
        }
        .eval(&Point { x: 1, y: 3 })
        .message,
        "Expected Point { x: 1, y: 2 }, got Point { x: 1, y: 3 }"
    );
}

#[test]
fn equal_custom_enum() {
    #[derive(Debug, PartialEq)]
    enum Color {
        Red,
        Rgb(u8, u8, u8),
    }
    assert!(Equal { value: Color::Red }.eval(&Color::Red).pass);
    assert_eq!(
        Equal { value: Color::Red }
            .eval(&Color::Rgb(0, 0, 0))
            .message,
        "Expected Red, got Rgb(0, 0, 0)"
    );
}

#[test]
fn equal_uses_custom_partial_eq() {
    // Case-insensitive equality.
    #[derive(Debug)]
    struct Ci(&'static str);
    impl PartialEq for Ci {
        fn eq(&self, other: &Self) -> bool {
            self.0.eq_ignore_ascii_case(other.0)
        }
    }
    assert!(Equal { value: Ci("HeLLo") }.eval(&Ci("hello")).pass);
    assert!(!Equal { value: Ci("hello") }.eval(&Ci("world")).pass);
}

#[test]
fn equal_uses_custom_debug_in_message() {
    #[derive(PartialEq)]
    struct Secret(u32);
    impl std::fmt::Debug for Secret {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "<redacted:{}>", self.0 % 10)
        }
    }
    assert_eq!(
        Equal { value: Secret(12) }.eval(&Secret(34)).message,
        "Expected <redacted:2>, got <redacted:4>"
    );
}

#[test]
fn equal_is_reusable_across_evaluations() {
    let e = Equal { value: 7 };
    assert!(e.eval(&7).pass);
    assert!(!e.eval(&8).pass);
    assert!(e.eval(&7).pass);
}

#[test]
fn equal_value_field_is_public() {
    let mut e = Equal { value: 1 };
    e.value = 2;
    assert!(e.eval(&2).pass);
}

// ---------------------------------------------------------------- Not

#[test]
fn not_of_passing_fails_describing_inner() {
    let r = Not {
        inner: Const(true, "inner"),
    }
    .eval(&0);
    assert!(!r.pass);
    assert_eq!(r.message, "Expected not inner, but it was");
}

#[test]
fn not_of_failing_passes_and_discards_inner_message() {
    let r = Not {
        inner: Const(false, "inner failure"),
    }
    .eval(&0);
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn not_of_always_false_always_passes() {
    for x in common::INTS {
        assert!(Not { inner: AlwaysFalse }.eval(&x).pass);
    }
}

#[test]
fn not_of_is_none() {
    let e = Not { inner: IsNone };
    assert!(!e.eval(&None::<i32>).pass);
    assert_eq!(
        e.eval(&None::<i32>).message,
        "Expected not None, but it was"
    );
    assert!(e.eval(&Some(3)).pass);
}

#[test]
fn not_of_equal() {
    let e = Not {
        inner: Equal { value: 3 },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x != 3);
    }
}

#[test]
fn double_not_restores_truth_and_describes_both_negations() {
    let e = Not {
        inner: Not {
            inner: Equal { value: 3 },
        },
    };
    assert!(e.eval(&3).pass);
    let r = e.eval(&4);
    assert!(!r.pass);
    assert_eq!(r.message, "Expected not not equal to 3, but it was");
}

#[test]
fn triple_not_matches_single_not() {
    let e = Not {
        inner: Not {
            inner: Not {
                inner: Equal { value: 3 },
            },
        },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x != 3);
    }
}

#[test]
fn not_evaluates_inner_exactly_once() {
    let (p, calls) = Probe::passing("x");
    let e = Not { inner: p };
    e.eval(&0);
    assert_eq!(calls.get(), 1);
    e.eval(&0);
    assert_eq!(calls.get(), 2);
}

#[test]
fn not_works_on_unsized_str() {
    let e = Not { inner: AlwaysFalse };
    assert!(<_ as EvalExt<str>>::eval(&e, "anything").pass);
}

// ---------------------------------------------------------------- And

#[test]
fn and_truth_table() {
    for (l, r) in [(false, false), (false, true), (true, false), (true, true)] {
        let res = And {
            left: Const(l, "L"),
            right: Const(r, "R"),
        }
        .eval(&());
        assert_eq!(res.pass, l && r, "{l} && {r}");
    }
}

#[test]
fn and_both_fail_reports_left_message() {
    let r = And {
        left: Const(false, "L"),
        right: Const(false, "R"),
    }
    .eval(&());
    assert_eq!(r.message, "L");
}

#[test]
fn and_left_fails_reports_left_message() {
    let r = And {
        left: Const(false, "L"),
        right: Const(true, "R"),
    }
    .eval(&());
    assert_eq!(r.message, "L");
}

#[test]
fn and_right_fails_reports_right_message() {
    let r = And {
        left: Const(true, "L"),
        right: Const(false, "R"),
    }
    .eval(&());
    assert_eq!(r.message, "R");
}

#[test]
fn and_pass_has_no_message() {
    let r = And {
        left: Const(true, "L note"),
        right: Const(true, "R note"),
    }
    .eval(&());
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn and_short_circuits_when_left_fails() {
    let (l, lc) = Probe::failing("L");
    let (r, rc) = Probe::passing("R");
    And { left: l, right: r }.check(&0);
    assert_eq!(lc.get(), 1);
    assert_eq!(rc.get(), 0, "right side must not be evaluated");
}

#[test]
fn and_evaluates_right_when_left_passes() {
    let (l, lc) = Probe::passing("L");
    let (r, rc) = Probe::failing("R");
    And { left: l, right: r }.check(&0);
    assert_eq!(lc.get(), 1);
    assert_eq!(rc.get(), 1);
}

#[test]
fn and_evaluates_left_before_right() {
    use std::cell::RefCell;
    use std::rc::Rc;
    struct Log(&'static str, Rc<RefCell<Vec<&'static str>>>);
    impl Expr<i32> for Log {
        fn check(&self, _: &i32) -> bool {
            self.1.borrow_mut().push(self.0);
            true
        }
    }
    let log = Rc::new(RefCell::new(vec![]));
    And {
        left: Log("left", log.clone()),
        right: Log("right", log.clone()),
    }
    .eval(&0);
    assert_eq!(*log.borrow(), ["left", "right"]);
}

#[test]
fn and_of_real_exprs() {
    let e = And {
        left: Not {
            inner: Equal { value: 1 },
        },
        right: Not {
            inner: Equal { value: 2 },
        },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x != 1 && x != 2);
    }
}

#[test]
fn and_with_always_false_always_fails() {
    let e = And {
        left: Not { inner: AlwaysFalse },
        right: AlwaysFalse,
    };
    let r = e.eval(&0);
    assert!(!r.pass);
    assert_eq!(r.message, ALWAYS_FALSE_MSG);
}

#[test]
fn and_deeply_nested_left_and_right() {
    // ((a && b) && (c && d))
    let e = And {
        left: And {
            left: Not {
                inner: Equal { value: 0 },
            },
            right: Not {
                inner: Equal { value: 1 },
            },
        },
        right: And {
            left: Not {
                inner: Equal { value: 2 },
            },
            right: Not {
                inner: Equal { value: 3 },
            },
        },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, !(0..=3).contains(&x));
    }
}

#[test]
fn and_on_unsized_str() {
    let e = And {
        left: Const(true, ""),
        right: Const(true, ""),
    };
    assert!(<_ as EvalExt<str>>::eval(&e, "s").pass);
}

#[test]
fn and_fields_are_public() {
    let e = And {
        left: Equal { value: 1 },
        right: Equal { value: 1 },
    };
    assert_eq!(e.left.value, 1);
    assert_eq!(e.right.value, 1);
}

// ---------------------------------------------------------------- Or

#[test]
fn or_truth_table() {
    for (l, r) in [(false, false), (false, true), (true, false), (true, true)] {
        let res = Or {
            left: Const(l, "L"),
            right: Const(r, "R"),
        }
        .eval(&());
        assert_eq!(res.pass, l || r, "{l} || {r}");
    }
}

#[test]
fn or_both_fail_combines_messages() {
    let r = Or {
        left: Const(false, "L"),
        right: Const(false, "R"),
    }
    .eval(&());
    assert!(!r.pass);
    assert_eq!(r.message, "Expected either:\n  - L\n  - R");
}

#[test]
fn or_left_pass_has_no_message() {
    let r = Or {
        left: Const(true, "left note"),
        right: Const(true, "right note"),
    }
    .eval(&());
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn or_right_pass_has_no_message() {
    let r = Or {
        left: Const(false, "L"),
        right: Const(true, "right note"),
    }
    .eval(&());
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn or_short_circuits_when_left_passes() {
    let (l, lc) = Probe::passing("L");
    let (r, rc) = Probe::passing("R");
    Or { left: l, right: r }.check(&0);
    assert_eq!(lc.get(), 1);
    assert_eq!(rc.get(), 0, "right side must not be evaluated");
}

#[test]
fn or_evaluates_right_when_left_fails() {
    let (l, lc) = Probe::failing("L");
    let (r, rc) = Probe::failing("R");
    Or { left: l, right: r }.check(&0);
    assert_eq!(lc.get(), 1);
    assert_eq!(rc.get(), 1);
}

#[test]
fn or_of_real_exprs() {
    let e = Or {
        left: Equal { value: 1 },
        right: Equal { value: 2 },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x == 1 || x == 2);
    }
    assert_eq!(
        e.eval(&3).message,
        "Expected either:\n  - Expected 1, got 3\n  - Expected 2, got 3"
    );
}

#[test]
fn or_nested_messages_are_flattened() {
    let e = Or {
        left: Or {
            left: Const(false, "a"),
            right: Const(false, "b"),
        },
        right: Const(false, "c"),
    };
    assert_eq!(e.eval(&()).message, "Expected either:\n  - a\n  - b\n  - c");
}

#[test]
fn or_right_nested_messages_are_flattened() {
    let e = Or {
        left: Const(false, "a"),
        right: Or {
            left: Const(false, "b"),
            right: Or {
                left: Const(false, "c"),
                right: Const(false, "d"),
            },
        },
    };
    assert_eq!(
        e.eval(&()).message,
        "Expected either:\n  - a\n  - b\n  - c\n  - d"
    );
}

#[test]
fn or_inside_and_inside_or_is_indented() {
    // The inner OR's failure comes back through the AND as one multi-line message,
    // which the outer OR indents under its bullet.
    let e = Or {
        left: And {
            left: Or {
                left: Const(false, "a"),
                right: Const(false, "b"),
            },
            right: Const(true, ""),
        },
        right: Const(false, "c"),
    };
    assert_eq!(
        e.eval(&()).message,
        "Expected either:\n  - Expected either:\n      - a\n      - b\n  - c"
    );
}

#[cfg(feature = "alloc")]
#[test]
fn or_flattens_through_references_and_boxes() {
    let inner = Or {
        left: Const(false, "a"),
        right: Const(false, "b"),
    };
    let boxed: Box<dyn Expr<()>> = Box::new(Or {
        left: Const(false, "c"),
        right: Const(false, "d"),
    });
    let e = Or {
        left: &inner,
        right: boxed,
    };
    assert_eq!(
        e.eval(&()).message,
        "Expected either:\n  - a\n  - b\n  - c\n  - d"
    );
}

#[test]
fn or_flattens_custom_alternatives_override() {
    // A user-defined OR-like expr can take part in flattening.
    use std::fmt::{self, Formatter};
    struct AnyOf(Vec<&'static str>);
    impl Expr<()> for AnyOf {
        fn check(&self, _: &()) -> bool {
            false
        }
        fn explain(&self, _: &(), f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0.join(" | "))
        }
        fn alternatives(&self) -> usize {
            self.0.len()
        }
        fn explain_alternatives(&self, _: &(), f: &mut Formatter<'_>) -> fmt::Result {
            for s in &self.0 {
                write!(f, "\n  - {s}")?;
            }
            Ok(())
        }
    }
    let e = Or {
        left: AnyOf(vec!["x", "y"]),
        right: Const(false, "z"),
    };
    assert_eq!(e.alternatives(), 3);
    assert_eq!(e.eval(&()).message, "Expected either:\n  - x\n  - y\n  - z");
    assert_eq!(AnyOf(vec!["x", "y"]).eval(&()).message, "x | y");
}

#[test]
fn alternatives_counts() {
    assert_eq!(<_ as Expr<i32>>::alternatives(&Equal { value: 1 }), 1);
    assert_eq!(<_ as Expr<i32>>::alternatives(&AlwaysFalse), 1);
    assert_eq!(<_ as Expr<i32>>::alternatives(&runit::NoAlternatives), 0);
    assert_eq!(
        <_ as Expr<i32>>::alternatives(&Not {
            inner: Or {
                left: AlwaysFalse,
                right: AlwaysFalse
            }
        }),
        1
    );
    assert_eq!(
        <_ as Expr<i32>>::alternatives(&And {
            left: Or {
                left: AlwaysFalse,
                right: AlwaysFalse
            },
            right: AlwaysFalse
        }),
        1
    );
    assert_eq!(
        <_ as Expr<i32>>::alternatives(&Or {
            left: Or {
                left: AlwaysFalse,
                right: runit::NoAlternatives
            },
            right: Or {
                left: AlwaysFalse,
                right: AlwaysFalse
            },
        }),
        3
    );
}

#[test]
fn precedence_of_primitives() {
    use runit::Precedence;
    assert_eq!(
        <_ as Expr<i32>>::precedence(&Equal { value: 1 }),
        Precedence::Atom
    );
    assert_eq!(
        <_ as Expr<i32>>::precedence(&Not { inner: AlwaysFalse }),
        Precedence::Atom
    );
    assert_eq!(
        <_ as Expr<i32>>::precedence(&And {
            left: AlwaysFalse,
            right: AlwaysFalse
        }),
        Precedence::And
    );
    assert_eq!(
        <_ as Expr<i32>>::precedence(&Or {
            left: AlwaysFalse,
            right: AlwaysFalse
        }),
        Precedence::Or
    );
    // An OR with an empty side takes the other side's precedence.
    assert_eq!(
        <_ as Expr<i32>>::precedence(&Or {
            left: runit::NoAlternatives,
            right: And {
                left: AlwaysFalse,
                right: AlwaysFalse
            },
        }),
        Precedence::And
    );
    assert!(Precedence::Or < Precedence::And && Precedence::And < Precedence::Atom);
}

#[test]
fn no_alternatives_contributes_nothing() {
    use runit::NoAlternatives;
    assert_eq!(<_ as Expr<i32>>::alternatives(&NoAlternatives), 0);
    assert_eq!(<_ as EvalExt<i32>>::describe_string(&NoAlternatives), "");
    let r = <_ as EvalExt<i32>>::eval(&NoAlternatives, &0);
    assert!(!r.pass);
    assert_eq!(r.message, "No alternatives to match.");

    let e = Or {
        left: NoAlternatives,
        right: Equal { value: 3 },
    };
    assert_eq!(e.eval(&4).message, "Expected 3, got 4");
    assert_eq!(e.describe_string(), "equal to 3");
    let e = Or {
        left: Equal { value: 3 },
        right: NoAlternatives,
    };
    assert_eq!(e.eval(&4).message, "Expected 3, got 4");
    let e = Or {
        left: NoAlternatives,
        right: NoAlternatives,
    };
    assert_eq!(
        <_ as EvalExt<i32>>::eval(&e, &0).message,
        "No alternatives to match."
    );
}

#[test]
fn explain_is_lazy_and_rechecks_only_on_failure() {
    // Checking never formats; explaining a failed AND re-checks the left side to find the culprit.
    let (l, lc) = Probe::passing("L");
    let (r, rc) = Probe::failing("R");
    let e = And { left: l, right: r };
    assert!(!e.check(&0));
    assert_eq!((lc.get(), rc.get()), (1, 1));
    assert_eq!(e.explanation(&0).to_string(), "R");
    assert_eq!((lc.get(), rc.get()), (2, 1));
}

#[test]
fn or_with_always_false_on_left_is_identity_for_truth() {
    let e = Or {
        left: AlwaysFalse,
        right: Equal { value: 3 },
    };
    for x in common::INTS {
        assert_eq!(e.eval(&x).pass, x == 3);
    }
    assert_eq!(
        e.eval(&4).message,
        "Expected either:\n  - Condition will always fail.\n  - Expected 3, got 4"
    );
}

#[test]
fn or_of_is_none_and_equal_on_options() {
    let e = Or {
        left: IsNone,
        right: Equal { value: Some(5) },
    };
    assert!(e.eval(&None).pass);
    assert!(e.eval(&Some(5)).pass);
    let r = e.eval(&Some(6));
    assert!(!r.pass);
    assert_eq!(
        r.message,
        "Expected either:\n  - Expected None, got Some(_)\n  - Expected Some(5), got Some(6)"
    );
}

#[test]
fn or_on_unsized_slice() {
    let e = Or {
        left: AlwaysFalse,
        right: Not { inner: AlwaysFalse },
    };
    assert!(<_ as EvalExt<[i32]>>::eval(&e, &[1, 2][..]).pass);
}

// ---------------------------------------------------------------- mixed

#[test]
fn de_morgan_not_and_equals_or_not() {
    for a in [false, true] {
        for b in [false, true] {
            let lhs = Not {
                inner: And {
                    left: Const(a, "a"),
                    right: Const(b, "b"),
                },
            }
            .eval(&());
            let rhs = Or {
                left: Not {
                    inner: Const(a, "a"),
                },
                right: Not {
                    inner: Const(b, "b"),
                },
            }
            .eval(&());
            assert_eq!(lhs.pass, rhs.pass, "a={a} b={b}");
        }
    }
}

#[test]
fn de_morgan_not_or_equals_and_not() {
    for a in [false, true] {
        for b in [false, true] {
            let lhs = Not {
                inner: Or {
                    left: Const(a, "a"),
                    right: Const(b, "b"),
                },
            }
            .eval(&());
            let rhs = And {
                left: Not {
                    inner: Const(a, "a"),
                },
                right: Not {
                    inner: Const(b, "b"),
                },
            }
            .eval(&());
            assert_eq!(lhs.pass, rhs.pass, "a={a} b={b}");
        }
    }
}

#[test]
fn distributivity_and_over_or_three_variable_truth_table() {
    for bits in 0u8..8 {
        let (a, b, c) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        let lhs = And {
            left: Const(a, "a"),
            right: Or {
                left: Const(b, "b"),
                right: Const(c, "c"),
            },
        }
        .eval(&());
        let rhs = Or {
            left: And {
                left: Const(a, "a"),
                right: Const(b, "b"),
            },
            right: And {
                left: Const(a, "a"),
                right: Const(c, "c"),
            },
        }
        .eval(&());
        assert_eq!(lhs.pass, a && (b || c));
        assert_eq!(lhs.pass, rhs.pass, "a={a} b={b} c={c}");
    }
}

#[test]
fn primitives_usable_as_dyn_expr() {
    let exprs: Vec<Box<dyn Expr<i32>>> = vec![
        Box::new(AlwaysFalse),
        Box::new(Equal { value: 1 }),
        Box::new(Not {
            inner: Equal { value: 1 },
        }),
        Box::new(And {
            left: Equal { value: 1 },
            right: Equal { value: 1 },
        }),
        Box::new(Or {
            left: Equal { value: 1 },
            right: Equal { value: 2 },
        }),
    ];
    let results: Vec<bool> = exprs.iter().map(|e| e.eval(&1).pass).collect();
    assert_eq!(results, [false, true, false, true, true]);
}

#[test]
fn large_balanced_tree_evaluates_correctly() {
    macro_rules! ne {
        ($v:expr) => {
            Not {
                inner: Equal { value: $v },
            }
        };
    }
    // Pass iff x not in 0..8
    let e = And {
        left: And {
            left: And {
                left: ne!(0),
                right: ne!(1),
            },
            right: And {
                left: ne!(2),
                right: ne!(3),
            },
        },
        right: And {
            left: And {
                left: ne!(4),
                right: ne!(5),
            },
            right: And {
                left: ne!(6),
                right: ne!(7),
            },
        },
    };
    for x in -20..=20 {
        assert_eq!(e.eval(&x).pass, !(0..8).contains(&x), "x = {x}");
    }
}
