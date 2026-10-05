#![allow(dead_code)]

use std::cell::Cell;
use std::fmt::{self, Formatter};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use runit::{Description, Explanation, Expr};

/// The outcome of evaluating an expression, with its failure message rendered eagerly.
/// Test-only convenience over `Expr::check` + `Expr::explanation`.
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub pass: bool,
    pub message: String,
}

pub trait EvalExt<T: ?Sized>: Expr<T> {
    /// Checks `actual`, rendering the explanation only on failure.
    fn eval(&self, actual: &T) -> Outcome {
        let pass = self.check(actual);
        let message = if pass {
            String::new()
        } else {
            Explanation::new(self, actual).to_string()
        };
        Outcome { pass, message }
    }

    fn describe_string(&self) -> String {
        Description::<Self, T>::new(self).to_string()
    }
}

impl<T: ?Sized, E: Expr<T> + ?Sized> EvalExt<T> for E {}

/// Runs `f`, expecting it to panic, and returns the panic message.
pub fn panic_message<F: FnOnce()>(f: F) -> String {
    let payload = catch_unwind(AssertUnwindSafe(f)).expect_err("expected closure to panic");
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        panic!("panic payload was neither String nor &str")
    }
}

/// Runs `f` and asserts it does not panic.
pub fn assert_no_panic<F: FnOnce()>(f: F) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(f)) {
        let msg = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        panic!("expected no panic, but panicked with: {msg}");
    }
}

/// An expression with a fixed outcome that counts how often it is checked.
pub struct Probe {
    pub pass: bool,
    pub message: &'static str,
    pub calls: Rc<Cell<usize>>,
}

impl Probe {
    pub fn passing(message: &'static str) -> (Self, Rc<Cell<usize>>) {
        Self::new(true, message)
    }

    pub fn failing(message: &'static str) -> (Self, Rc<Cell<usize>>) {
        Self::new(false, message)
    }

    pub fn new(pass: bool, message: &'static str) -> (Self, Rc<Cell<usize>>) {
        let calls = Rc::new(Cell::new(0));
        (
            Self {
                pass,
                message,
                calls: calls.clone(),
            },
            calls,
        )
    }
}

impl<T: ?Sized> Expr<T> for Probe {
    fn check(&self, _actual: &T) -> bool {
        self.calls.set(self.calls.get() + 1);
        self.pass
    }

    fn explain(&self, _actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

/// An expression with a fixed outcome and message.
pub struct Const(pub bool, pub &'static str);

impl<T: ?Sized> Expr<T> for Const {
    fn check(&self, _actual: &T) -> bool {
        self.0
    }

    fn explain(&self, _actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }
}

/// Values used to exhaustively check `i32` expressions against an oracle.
pub const INTS: std::ops::RangeInclusive<i32> = -10..=10;

/// Checks `expr` against `oracle` for every value in [`INTS`].
pub fn check_ints<E: Expr<i32>>(expr: E, oracle: impl Fn(i32) -> bool) {
    for x in INTS {
        let r = expr.eval(&x);
        assert_eq!(
            r.pass,
            oracle(x),
            "mismatch for {x}: expr.pass = {}, message = {:?}",
            r.pass,
            r.message
        );
        if r.pass {
            assert!(
                r.message.is_empty(),
                "passing result for {x} carried message {:?}",
                r.message
            );
        } else {
            assert!(
                !r.message.is_empty(),
                "failing result for {x} had empty message"
            );
        }
    }
}

/// Checks `expr` against `oracle` for `None` and a spread of `Some` values.
pub fn check_options<E: Expr<Option<i32>>>(expr: E, oracle: impl Fn(Option<i32>) -> bool) {
    let values = std::iter::once(None).chain((-5..=10).map(Some));
    for x in values {
        let r = expr.eval(&x);
        assert_eq!(
            r.pass,
            oracle(x),
            "mismatch for {x:?}: expr.pass = {}, message = {:?}",
            r.pass,
            r.message
        );
        if !r.pass {
            assert!(
                !r.message.is_empty(),
                "failing result for {x:?} had empty message"
            );
        }
    }
}
