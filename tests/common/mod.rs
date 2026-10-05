#![allow(dead_code)]

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use runit::{Expr, ExprResult};

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

/// An expression with a fixed outcome that counts how often it is evaluated.
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
    fn eval(&self, _actual: &T) -> ExprResult {
        self.calls.set(self.calls.get() + 1);
        ExprResult {
            pass: self.pass,
            message: self.message.to_string(),
        }
    }

    fn describe(&self) -> String {
        self.message.to_string()
    }
}

/// An expression with a fixed outcome and message.
pub struct Const(pub bool, pub &'static str);

impl<T: ?Sized> Expr<T> for Const {
    fn eval(&self, _actual: &T) -> ExprResult {
        ExprResult {
            pass: self.0,
            message: self.1.to_string(),
        }
    }

    fn describe(&self) -> String {
        self.1.to_string()
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
