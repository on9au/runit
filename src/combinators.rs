use core::fmt::{self, Formatter};

use crate::{Expr, Precedence};

pub struct Not<A> {
    pub inner: A,
}
pub struct And<A, B> {
    pub left: A,
    pub right: B,
}
pub struct Or<A, B> {
    pub left: A,
    pub right: B,
}

impl<A, B, T> Expr<T> for And<A, B>
where
    T: ?Sized,
    A: Expr<T>,
    B: Expr<T>,
{
    fn check(&self, actual: &T) -> bool {
        self.left.check(actual) && self.right.check(actual)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.left.check(actual) {
            self.left.explain(actual, f)
        } else {
            self.right.explain(actual, f)
        }
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        describe_grouped(&self.left, Precedence::And, f)?;
        f.write_str(" and ")?;
        describe_grouped(&self.right, Precedence::And, f)
    }

    fn precedence(&self) -> Precedence {
        Precedence::And
    }
}

impl<A, B, T> Expr<T> for Or<A, B>
where
    T: ?Sized,
    A: Expr<T>,
    B: Expr<T>,
{
    fn check(&self, actual: &T) -> bool {
        self.left.check(actual) || self.right.check(actual)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        explain_or(&self.left, &self.right, actual, f)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        describe_or(&self.left, &self.right, f)
    }

    fn precedence(&self) -> Precedence {
        precedence_or(&self.left, &self.right)
    }

    fn alternatives(&self) -> usize {
        self.left.alternatives() + self.right.alternatives()
    }

    fn explain_alternatives(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        self.left.explain_alternatives(actual, f)?;
        self.right.explain_alternatives(actual, f)
    }
}

impl<A, T> Expr<T> for Not<A>
where
    T: ?Sized,
    A: Expr<T>,
{
    fn check(&self, actual: &T) -> bool {
        !self.inner.check(actual)
    }

    fn explain(&self, _actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Expected ")?;
        self.describe(f)?;
        f.write_str(", but it was")
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("not ")?;
        describe_grouped(&self.inner, Precedence::Atom, f)
    }
}

/// Explains a failed OR as a flat list of its alternatives, or as the single
/// alternative itself when the other side is empty.
pub(crate) fn explain_or<T, L, R>(
    left: &L,
    right: &R,
    actual: &T,
    f: &mut Formatter<'_>,
) -> fmt::Result
where
    T: ?Sized,
    L: Expr<T>,
    R: Expr<T>,
{
    match (left.alternatives(), right.alternatives()) {
        (0, 0) => f.write_str("No alternatives to match."),
        (0, _) => right.explain(actual, f),
        (_, 0) => left.explain(actual, f),
        _ => {
            f.write_str("Expected either:")?;
            left.explain_alternatives(actual, f)?;
            right.explain_alternatives(actual, f)
        }
    }
}

/// Describes an OR, skipping empty sides.
pub(crate) fn describe_or<T, L, R>(left: &L, right: &R, f: &mut Formatter<'_>) -> fmt::Result
where
    T: ?Sized,
    L: Expr<T>,
    R: Expr<T>,
{
    match (left.alternatives(), right.alternatives()) {
        (0, _) => right.describe(f),
        (_, 0) => left.describe(f),
        _ => {
            left.describe(f)?;
            f.write_str(" or ")?;
            right.describe(f)
        }
    }
}

pub(crate) fn precedence_or<T, L, R>(left: &L, right: &R) -> Precedence
where
    T: ?Sized,
    L: Expr<T>,
    R: Expr<T>,
{
    match (left.alternatives(), right.alternatives()) {
        (0, _) => right.precedence(),
        (_, 0) => left.precedence(),
        _ => Precedence::Or,
    }
}

/// Describes `expr`, parenthesized if it binds looser than `min`.
pub(crate) fn describe_grouped<T, E>(
    expr: &E,
    min: Precedence,
    f: &mut Formatter<'_>,
) -> fmt::Result
where
    T: ?Sized,
    E: Expr<T>,
{
    if expr.precedence() < min {
        f.write_str("(")?;
        expr.describe(f)?;
        f.write_str(")")
    } else {
        expr.describe(f)
    }
}
