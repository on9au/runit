use core::fmt::{self, Formatter, Write};

use crate::display::{Description, Explanation, Indented};

/// How tightly a description binds, used to parenthesize nested descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Or,
    And,
    Atom,
}

/// A condition on a `T`.
///
/// Checking never allocates: failure messages are written lazily, straight into a
/// [`Formatter`], and only when something actually displays them.
pub trait Expr<T: ?Sized> {
    /// Whether `actual` satisfies the expression.
    fn check(&self, actual: &T) -> bool;

    /// Writes why `actual` fails. Only meaningful after [`check`](Expr::check) returned `false`.
    fn explain(&self, _actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Expected ")?;
        self.describe(f)
    }

    /// Writes a short description of what the expression expects, e.g. `equal to 1`.
    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(core::any::type_name::<Self>())
    }

    /// How tightly [`describe`](Expr::describe)'s output binds.
    fn precedence(&self) -> Precedence {
        Precedence::Atom
    }

    /// Number of top-level OR alternatives, so nested ORs can report one flat list.
    ///
    /// Only OR-like expressions need to override this, together with
    /// [`explain_alternatives`](Expr::explain_alternatives).
    fn alternatives(&self) -> usize {
        1
    }

    /// Writes each failed alternative as a list item. Only meaningful after
    /// [`check`](Expr::check) returned `false`.
    fn explain_alternatives(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n  - ")?;
        write!(Indented(f), "{}", Explanation::new(self, actual))
    }
}

/// Conveniences for checking and displaying any [`Expr`], including trait objects.
///
/// Blanket-implemented for every expression, sized or not, so `rule.validate(x)` works the
/// same on `Is::equal_to(1)`, a `&dyn Expr<T>` and a `Box<dyn Expr<T>>`. These live outside
/// [`Expr`] because on a trait object the trait's own methods would need `Self: Sized`.
/// Bring it into scope with `use runit::prelude::*`.
pub trait ExprExt<T: ?Sized>: Expr<T> {
    /// Checks `actual`, returning why it fails as the error.
    ///
    /// The error borrows the expression and the value, so nothing is formatted or
    /// allocated unless the caller displays it.
    fn validate<'a>(&'a self, actual: &'a T) -> Result<(), Explanation<'a, Self, T>> {
        if self.check(actual) {
            Ok(())
        } else {
            Err(Explanation::new(self, actual))
        }
    }

    /// Displays why `actual` fails.
    fn explanation<'a>(&'a self, actual: &'a T) -> Explanation<'a, Self, T> {
        Explanation::new(self, actual)
    }

    /// Displays what the expression expects.
    fn description(&self) -> Description<'_, Self, T> {
        Description::new(self)
    }
}

impl<T: ?Sized, E: Expr<T> + ?Sized> ExprExt<T> for E {}

macro_rules! forward_expr {
    ($ptr:ty) => {
        impl<T: ?Sized, E: Expr<T> + ?Sized> Expr<T> for $ptr {
            fn check(&self, actual: &T) -> bool {
                (**self).check(actual)
            }

            fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).explain(actual, f)
            }

            fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).describe(f)
            }

            fn precedence(&self) -> Precedence {
                (**self).precedence()
            }

            fn alternatives(&self) -> usize {
                (**self).alternatives()
            }

            fn explain_alternatives(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).explain_alternatives(actual, f)
            }
        }
    };
}

forward_expr!(&E);
#[cfg(feature = "alloc")]
forward_expr!(alloc::boxed::Box<E>);
#[cfg(feature = "alloc")]
forward_expr!(alloc::rc::Rc<E>);
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
forward_expr!(alloc::sync::Arc<E>);
