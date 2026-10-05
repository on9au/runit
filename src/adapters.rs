use core::fmt::{self, Debug, Display, Formatter};

use crate::combinators::describe_grouped;
use crate::{Expr, Precedence};

/// Matches when `pred` returns `true`, described as `description`.
pub struct Satisfies<D, F> {
    pub description: D,
    pub pred: F,
}

impl<T, D, F> Expr<T> for Satisfies<D, F>
where
    T: Debug + ?Sized,
    D: Display,
    F: Fn(&T) -> bool,
{
    fn check(&self, actual: &T) -> bool {
        (self.pred)(actual)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Expected {}, got {:?}", self.description, actual)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

/// Matches when the part of the value borrowed by `get` matches `expr`.
pub struct Field<N, F, E> {
    pub name: N,
    pub get: F,
    pub expr: E,
}

impl<T, U, N, F, E> Expr<T> for Field<N, F, E>
where
    T: ?Sized,
    U: ?Sized,
    N: Display,
    F: Fn(&T) -> &U,
    E: Expr<U>,
{
    fn check(&self, actual: &T) -> bool {
        self.expr.check((self.get)(actual))
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.name)?;
        self.expr.explain((self.get)(actual), f)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.name)?;
        describe_grouped(&self.expr, Precedence::Atom, f)
    }
}

/// Matches when the value computed by `get` matches `expr`.
pub struct Property<N, F, E> {
    pub name: N,
    pub get: F,
    pub expr: E,
}

impl<T, U, N, F, E> Expr<T> for Property<N, F, E>
where
    T: ?Sized,
    N: Display,
    F: Fn(&T) -> U,
    E: Expr<U>,
{
    fn check(&self, actual: &T) -> bool {
        self.expr.check(&(self.get)(actual))
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.name)?;
        self.expr.explain(&(self.get)(actual), f)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.name)?;
        describe_grouped(&self.expr, Precedence::Atom, f)
    }
}

/// Describes `expr` as `name`, so it reads as one rule rather than its full expansion.
pub struct Named<N, E> {
    pub name: N,
    pub expr: E,
}

impl<T, N, E> Expr<T> for Named<N, E>
where
    T: ?Sized,
    N: Display,
    E: Expr<T>,
{
    fn check(&self, actual: &T) -> bool {
        self.expr.check(actual)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.name)?;
        self.expr.explain(actual, f)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
