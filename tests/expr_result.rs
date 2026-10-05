use runit::ExprResult;

#[test]
fn pass_has_pass_true_and_empty_message() {
    let r = ExprResult::pass();
    assert!(r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn fail_has_pass_false_and_given_message() {
    let r = ExprResult::fail("nope");
    assert!(!r.pass);
    assert_eq!(r.message, "nope");
}

#[test]
fn fail_accepts_str() {
    assert_eq!(ExprResult::fail("a").message, "a");
}

#[test]
fn fail_accepts_owned_string() {
    assert_eq!(ExprResult::fail(String::from("owned")).message, "owned");
}

#[test]
fn fail_accepts_formatted_string() {
    let n = 42;
    assert_eq!(ExprResult::fail(format!("n = {n}")).message, "n = 42");
}

#[test]
fn fail_accepts_char() {
    assert_eq!(ExprResult::fail('x').message, "x");
}

#[test]
fn fail_accepts_boxed_str() {
    let b: Box<str> = "boxed".into();
    assert_eq!(ExprResult::fail(b).message, "boxed");
}

#[test]
fn fail_accepts_cow() {
    let c: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed("cow");
    assert_eq!(ExprResult::fail(c).message, "cow");
}

#[test]
fn fail_allows_empty_message() {
    let r = ExprResult::fail("");
    assert!(!r.pass);
    assert_eq!(r.message, "");
}

#[test]
fn fail_preserves_multiline_and_unicode() {
    let msg = "line one\nline two\t🦀 ünïcödé";
    assert_eq!(ExprResult::fail(msg).message, msg);
}

#[test]
fn can_be_constructed_with_struct_literal() {
    let r = ExprResult {
        pass: true,
        message: "custom pass note".into(),
    };
    assert!(r.pass);
    assert_eq!(r.message, "custom pass note");
}

#[test]
fn fields_are_mutable() {
    let mut r = ExprResult::pass();
    r.pass = false;
    r.message.push_str("changed");
    assert!(!r.pass);
    assert_eq!(r.message, "changed");
}
