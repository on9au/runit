//! Tests for closure, field, property and named adapters.

mod common;

use common::{EvalExt, check_ints};
use runit::{Expr, Is, Named};

struct User {
    name: String,
    age: u32,
    roles: Vec<&'static str>,
}

fn user(name: &str, age: u32, roles: &[&'static str]) -> User {
    User {
        name: name.into(),
        age,
        roles: roles.to_vec(),
    }
}

#[test]
fn satisfies_runs_the_predicate() {
    check_ints(Is::satisfies("even", |x: &i32| x % 2 == 0), |x| x % 2 == 0);
}

#[test]
fn satisfies_uses_its_description() {
    let e = Is::satisfies("even", |x: &i32| x % 2 == 0).or().equal_to(7);
    assert_eq!(EvalExt::<i32>::describe_string(&e), "even or equal to 7");
    assert_eq!(
        e.eval(&3).message,
        "Expected either:\n  - Expected even, got 3\n  - Expected 7, got 3"
    );
}

#[test]
fn satisfies_on_unsized_targets() {
    let e = Is::satisfies("ascii", |s: &str| s.is_ascii());
    assert!(<_ as EvalExt<str>>::eval(&e, "abc").pass);
    assert!(!<_ as EvalExt<str>>::eval(&e, "é").pass);
}

#[test]
fn field_projects_borrowed_parts() {
    let adult = Is::field("age", |u: &User| &u.age, Is::at_least(18));
    assert!(adult.check(&user("a", 30, &[])));
    assert_eq!(
        adult.eval(&user("a", 12, &[])).message,
        "age: Expected at least 18, got 12"
    );
}

#[test]
fn field_can_project_to_unsized() {
    let e = Is::field("name", |u: &User| u.name.as_str(), Is::starts_with("A"));
    assert!(e.check(&user("Ann", 1, &[])));
    assert!(!e.check(&user("Bob", 1, &[])));
}

#[test]
fn property_projects_computed_values() {
    let e = Is::property("name length", |u: &User| u.name.len(), Is::at_most(3));
    assert!(e.check(&user("Ann", 1, &[])));
    assert_eq!(
        e.eval(&user("Annabel", 1, &[])).message,
        "name length: Expected at most 3, got 7"
    );
}

#[test]
fn projections_describe_with_grouping() {
    let e = Is::field("age", |u: &User| &u.age, Is::at_least(18).and().at_most(65))
        .and()
        .field("roles", |u: &User| &u.roles, Is::contains("admin"));
    assert_eq!(
        EvalExt::<User>::describe_string(&e),
        r#"age (at least 18 and at most 65) and roles containing "admin""#
    );
}

#[test]
fn named_replaces_description_but_keeps_reason() {
    let e = Is::at_least(18).named("adult");
    assert_eq!(EvalExt::<u32>::describe_string(&e), "adult");
    assert_eq!(e.eval(&3u32).message, "adult: Expected at least 18, got 3");
}

#[test]
fn named_chain_continues() {
    let e = Is::at_least(0)
        .and()
        .at_most(9)
        .named("digit")
        .or()
        .equal_to(42);
    check_ints(e, |x| (0..=9).contains(&x) || x == 42);
    let e = Is::at_least(0)
        .and()
        .at_most(9)
        .named("digit")
        .or()
        .equal_to(42);
    assert_eq!(EvalExt::<i32>::describe_string(&e), "digit or equal to 42");
}

#[test]
fn negated_named_rule() {
    let e = Is::not().named("banned", Is::contains("banned"));
    let u = vec!["banned"];
    assert_eq!(e.eval(&u).message, "Expected not banned, but it was");
}

#[test]
fn policy_example() {
    let is_owner = Named {
        name: "owner",
        expr: Is::field(
            "name",
            |u: &User| &u.name,
            Is::equal_to(String::from("root")),
        ),
    };
    let can_edit = Is::field("roles", |u: &User| &u.roles, Is::contains("admin"))
        .named("admin")
        .or()
        .matches(is_owner)
        .and()
        .field("age", |u: &User| &u.age, Is::at_least(18));

    assert!(can_edit.check(&user("x", 20, &["admin"])));
    assert!(can_edit.check(&user("root", 20, &[])));
    assert!(!can_edit.check(&user("root", 10, &[])));
    assert_eq!(
        EvalExt::<User>::describe_string(&can_edit),
        "admin or owner and age at least 18"
    );
    assert_eq!(
        can_edit.eval(&user("x", 20, &[])).message,
        "Expected either:\n  - admin: roles: Expected collection containing \"admin\", but it did not\n  - owner: name: Expected \"root\", got \"x\""
    );
}

// ---------------------------------------------------------------- lifetimes

struct Request<'a> {
    user: &'a User,
    target: &'a str,
}

fn user_age<'a>(r: &'a Request<'_>) -> &'a u32 {
    &r.user.age
}

/// Rules over a type with a lifetime parameter, valid for every lifetime.
fn adult_self_service() -> impl for<'a> Expr<Request<'a>> {
    Is::satisfies("self service", |r: &Request| r.target == r.user.name)
        .and()
        .property(
            "target length",
            |r: &Request| r.target.len(),
            Is::at_most(8),
        )
        .and()
        .field("age", user_age, Is::at_least(18))
}

impl std::fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.user.name, self.target)
    }
}

#[test]
fn higher_ranked_rules_over_borrowed_types() {
    let rule = adult_self_service();
    let ann = user("ann", 30, &[]);
    let kid = user("kid", 9, &[]);
    for (u, target, pass) in [
        (&ann, "ann", true),
        (&ann, "bob", false),
        (&kid, "kid", false),
    ] {
        let target = String::from(target);
        assert_eq!(
            rule.check(&Request {
                user: u,
                target: &target
            }),
            pass
        );
    }
    let target = String::from("bob");
    assert_eq!(
        rule.eval(&Request {
            user: &ann,
            target: &target
        })
        .message,
        "Expected self service, got ann -> bob"
    );
}

// ---------------------------------------------------------------- one_of

#[test]
fn one_of_compares_across_types() {
    let e = Is::one_of(["debug", "info"]);
    assert!(e.eval(&String::from("info")).pass);
    assert!(e.eval(&"debug").pass);
    assert_eq!(
        e.eval(&String::from("loud")).message,
        r#"Expected one of ["debug", "info"], got "loud""#
    );
    assert_eq!(
        EvalExt::<&str>::describe_string(&e),
        r#"one of ["debug", "info"]"#
    );
}

#[test]
fn one_of_accepts_slices_and_vecs() {
    let allowed: &[i32] = &[1, 3, 5];
    check_ints(Is::one_of(allowed), |x| [1, 3, 5].contains(&x));
    check_ints(Is::not().one_of(vec![0, 2]), |x| x != 0 && x != 2);
    assert!(!Is::one_of(Vec::<i32>::new()).eval(&1).pass);
}
