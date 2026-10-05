//! A small access-policy engine built from runit expressions.
//!
//! Each policy is an `Expr<Request>`: it decides whether a user may perform an action on a
//! document, describes itself in plain English, and explains every denial.
//!
//! Run with `cargo run --example access_policy`.

use runit::{Description, Expr, Is};

struct User {
    name: &'static str,
    email: &'static str,
    roles: Vec<&'static str>,
    account_age_days: u32,
    suspended: bool,
}

struct Doc {
    title: &'static str,
    owner: &'static str,
    public: bool,
    locked: bool,
}

/// Everything a policy gets to look at.
struct Request<'a> {
    user: &'a User,
    doc: &'a Doc,
}

// ------------------------------------------------------------------ building blocks

fn admin<'a>() -> impl Expr<Request<'a>> {
    Is::field("roles", |r: &Request| &r.user.roles, Is::contains("admin")).named("admin")
}

fn owner<'a>() -> impl Expr<Request<'a>> {
    Is::satisfies("owner", |r: &Request| r.doc.owner == r.user.name)
}

fn doc_locked<'a>() -> impl Expr<Request<'a>> {
    Is::field(
        "doc.locked",
        |r: &Request| &r.doc.locked,
        Is::equal_to(true),
    )
    .named("doc locked")
}

fn staff<'a>() -> impl Expr<Request<'a>> {
    Is::field(
        "email",
        |r: &Request| &r.user.email,
        Is::ends_with("@acme.io"),
    )
    .named("staff")
}

fn in_good_standing<'a>() -> impl Expr<Request<'a>> {
    Is::field(
        "suspended",
        |r: &Request| &r.user.suspended,
        Is::equal_to(false),
    )
    .named("not suspended")
}

// ------------------------------------------------------------------ policies

fn read_doc<'a>() -> impl Expr<Request<'a>> {
    Is::field(
        "doc.public",
        |r: &Request| &r.doc.public,
        Is::equal_to(true),
    )
    .named("doc public")
    .or()
    .matches(staff())
    .or()
    .matches(owner())
}

fn edit_doc<'a>() -> impl Expr<Request<'a>> {
    // Admins may edit locked docs; owners may not.
    let admin_or_owner = Is::matches(admin())
        .or()
        .matches(owner())
        .and()
        .not()
        .matches(doc_locked());
    Is::matches(in_good_standing())
        .and()
        .matches(admin_or_owner)
}

fn delete_doc<'a>() -> impl Expr<Request<'a>> {
    Is::matches(admin())
        .and()
        .field(
            "account age (days)",
            |r: &Request| &r.user.account_age_days,
            Is::at_least(30),
        )
        .and()
        .not()
        .matches(doc_locked())
}

fn rename_doc<'a>() -> impl Expr<Request<'a>> {
    Is::matches(edit_doc()).named("can edit").and().field(
        "doc.title",
        |r: &Request| &r.doc.title,
        Is::not().starts_with("[archived]"),
    )
}

// ------------------------------------------------------------------ demo

fn main() {
    let alice = User {
        name: "alice",
        email: "alice@acme.io",
        roles: vec!["admin", "editor"],
        account_age_days: 400,
        suspended: false,
    };
    let bob = User {
        name: "bob",
        email: "bob@gmail.com",
        roles: vec!["editor"],
        account_age_days: 12,
        suspended: false,
    };
    let carol = User {
        name: "carol",
        email: "carol@acme.io",
        roles: vec![],
        account_age_days: 90,
        suspended: true,
    };

    let report = Doc {
        title: "Q3 report",
        owner: "bob",
        public: false,
        locked: false,
    };
    let handbook = Doc {
        title: "[archived] Handbook",
        owner: "carol",
        public: true,
        locked: true,
    };

    let policies: [(&str, &dyn Expr<Request>); 4] = [
        ("read", &read_doc()),
        ("edit", &edit_doc()),
        ("rename", &rename_doc()),
        ("delete", &delete_doc()),
    ];

    println!("POLICIES");
    for (action, policy) in &policies {
        println!("  {action:<7} {}", Description::new(*policy));
    }

    for doc in [&report, &handbook] {
        println!("\n{} (owner: {})", doc.title, doc.owner);
        for user in [&alice, &bob, &carol] {
            let request = Request { user, doc };
            for (action, policy) in &policies {
                match policy.validate(&request) {
                    Ok(()) => println!("  {:<6} {action:<7} ALLOWED", user.name),
                    Err(why) => {
                        println!("  {:<6} {action:<7} DENIED", user.name);
                        println!("{}", indent(&why.to_string(), 4));
                    }
                }
            }
        }
    }
}

fn indent(text: &str, by: usize) -> String {
    let pad = " ".repeat(by);
    text.lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
