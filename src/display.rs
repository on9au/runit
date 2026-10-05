use core::fmt::{self, Debug, Display, Formatter, Write};
use core::marker::PhantomData;

use crate::Expr;

/// Displays why a value fails an expression. See [`ExprExt::explanation`](crate::ExprExt::explanation).
pub struct Explanation<'a, E: ?Sized, T: ?Sized> {
    expr: &'a E,
    actual: &'a T,
}

impl<'a, E: ?Sized, T: ?Sized> Explanation<'a, E, T> {
    pub fn new(expr: &'a E, actual: &'a T) -> Self {
        Self { expr, actual }
    }
}

impl<E: Expr<T> + ?Sized, T: ?Sized> Display for Explanation<'_, E, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.expr.explain(self.actual, f)
    }
}

impl<E: Expr<T> + ?Sized, T: ?Sized> Debug for Explanation<'_, E, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl<E: Expr<T> + ?Sized, T: ?Sized> core::error::Error for Explanation<'_, E, T> {}

/// Displays what an expression expects. See [`ExprExt::description`](crate::ExprExt::description).
pub struct Description<'a, E: ?Sized, T: ?Sized> {
    expr: &'a E,
    actual: PhantomData<fn(&T)>,
}

impl<'a, E: ?Sized, T: ?Sized> Description<'a, E, T> {
    pub fn new(expr: &'a E) -> Self {
        Self {
            expr,
            actual: PhantomData,
        }
    }
}

impl<E: Expr<T> + ?Sized, T: ?Sized> Display for Description<'_, E, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.expr.describe(f)
    }
}

impl<E: Expr<T> + ?Sized, T: ?Sized> Debug for Description<'_, E, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// Indents every line after the first, so multi-line messages sit under a list bullet.
pub(crate) struct Indented<'a, 'b>(pub(crate) &'a mut Formatter<'b>);

impl Write for Indented<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut lines = s.split('\n');
        if let Some(first) = lines.next() {
            self.0.write_str(first)?;
        }
        for line in lines {
            self.0.write_str("\n    ")?;
            self.0.write_str(line)?;
        }
        Ok(())
    }
}
