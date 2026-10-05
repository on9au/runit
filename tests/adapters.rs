//! Tests for the `Explanation` and `Description` display adapters.

use std::fmt::{self, Formatter};

use runit::{AlwaysFalse, Description, Equal, Explanation, Expr, ExprExt, Is, IsNone, Not};

#[test]
fn explanation_displays_failure_message() {
    let e = Is::equal_to(5);
    assert_eq!(e.explanation(&3).to_string(), "Expected 5, got 3");
}

#[test]
fn explanation_debug_matches_display() {
    let e = Is::equal_to(5);
    assert_eq!(format!("{:?}", e.explanation(&3)), "Expected 5, got 3");
}

#[test]
fn explanation_embeds_in_format_strings() {
    let e = Is::none();
    assert_eq!(
        format!("[{}]", e.explanation(&Some(1))),
        "[Expected None, got Some(_)]"
    );
}

#[test]
fn explanation_new_works_for_dyn_exprs() {
    let boxed: Box<dyn Expr<i32>> = Box::new(Is::not().equal_to(1));
    assert_eq!(
        Explanation::new(&*boxed, &1).to_string(),
        "Expected not equal to 1, but it was"
    );
}

#[test]
fn explanation_and_description_work_directly_on_dyn() {
    let e: &dyn Expr<i32> = &Equal { value: 2 };
    assert_eq!(e.explanation(&1).to_string(), "Expected 2, got 1");
    assert_eq!(e.description().to_string(), "equal to 2");
}

#[test]
fn explanation_works_for_unsized_actual() {
    struct NonEmpty;
    impl Expr<str> for NonEmpty {
        fn check(&self, actual: &str) -> bool {
            !actual.is_empty()
        }
        fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("a non-empty string")
        }
    }
    assert_eq!(
        NonEmpty.explanation("").to_string(),
        "Expected a non-empty string"
    );
}

#[test]
fn explanation_is_rendered_on_every_display() {
    // Nothing is cached: each display re-runs `explain`.
    let e = Is::equal_to(1);
    let x = e.explanation(&2);
    assert_eq!(x.to_string(), x.to_string());
}

#[test]
fn description_displays_expectation() {
    assert_eq!(
        Is::equal_to(1)
            .or()
            .not()
            .equal_to(2)
            .description()
            .to_string(),
        "equal to 1 or not equal to 2"
    );
}

#[test]
fn description_debug_matches_display() {
    let d = Not {
        inner: Equal { value: 1 },
    };
    let d = <_ as ExprExt<i32>>::description(&d);
    assert_eq!(format!("{d:?}"), "not equal to 1");
}

#[test]
fn description_new_needs_target_type_for_generic_exprs() {
    assert_eq!(
        Description::<_, Option<u8>>::new(&IsNone).to_string(),
        "None"
    );
    assert_eq!(
        Description::<_, str>::new(&AlwaysFalse).to_string(),
        "always false"
    );
}

#[test]
fn default_explain_uses_describe() {
    struct Positive;
    impl Expr<i32> for Positive {
        fn check(&self, actual: &i32) -> bool {
            *actual > 0
        }
        fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("positive")
        }
    }
    assert_eq!(Positive.explanation(&-1).to_string(), "Expected positive");
}

#[test]
fn default_explain_and_describe_fall_back_to_type_name() {
    struct Bare;
    impl Expr<i32> for Bare {
        fn check(&self, _: &i32) -> bool {
            false
        }
    }
    let d = Bare.description().to_string();
    assert!(d.ends_with("Bare"), "{d}");
    assert_eq!(Bare.explanation(&0).to_string(), format!("Expected {d}"));
}

#[test]
fn formatter_errors_propagate() {
    struct Failing;
    impl fmt::Write for Failing {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }
    use std::fmt::Write;
    assert!(write!(Failing, "{}", Is::equal_to(1).explanation(&2)).is_err());
    assert!(write!(Failing, "{}", Is::equal_to(1).description()).is_err());
    assert!(
        write!(
            Failing,
            "{}",
            Is::equal_to(1).or().equal_to(2).explanation(&3)
        )
        .is_err()
    );
}
