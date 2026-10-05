use core::fmt::{self, Formatter};

use crate::combinators::{describe_or, explain_or, precedence_or};
use crate::{
    AllItems, And, AnyItem, AtLeast, AtMost, Contains, ContainsStr, EndsWith, Equal, Expr, Field,
    GreaterThan, HasLength, InRange, IsEmpty, IsNone, LessThan, Named, NoAlternatives, Not, Or,
    Precedence, Property, Satisfies, StartsWith,
};

/// Query
///
/// - `p` (`<P>`) - Finished OR components.
/// - `g` (`<G>`) - AND-group currently being built.
pub struct Query<P, G> {
    p: P,
    g: G,
}
impl<P, G> Query<P, G> {
    pub fn or(self) -> PendingOr<P, G> {
        PendingOr { resulting_p: self }
    }
    pub fn and(self) -> PendingAnd<P, G> {
        PendingAnd { resulting_p: self }
    }

    /// Wraps everything built so far as one rule called `name`, e.g.
    /// `Is::at_least(18).named("adult")`. The chain can continue after it.
    pub fn named<N>(self, name: N) -> Query<NoAlternatives, Named<N, Self>> {
        Start.then(Named { name, expr: self })
    }
}

impl<T: ?Sized, P: Expr<T>, G: Expr<T>> Expr<T> for Query<P, G> {
    fn check(&self, actual: &T) -> bool {
        self.p.check(actual) || self.g.check(actual)
    }

    fn explain(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        explain_or(&self.p, &self.g, actual, f)
    }

    fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
        describe_or(&self.p, &self.g, f)
    }

    fn precedence(&self) -> Precedence {
        precedence_or(&self.p, &self.g)
    }

    fn alternatives(&self) -> usize {
        self.p.alternatives() + self.g.alternatives()
    }

    fn explain_alternatives(&self, actual: &T, f: &mut Formatter<'_>) -> fmt::Result {
        self.p.explain_alternatives(actual, f)?;
        self.g.explain_alternatives(actual, f)
    }
}

pub struct PendingOr<P, G> {
    resulting_p: Query<P, G>,
}
pub struct PendingAnd<P, G> {
    resulting_p: Query<P, G>,
}

pub struct Is;

/// The head of a builder chain, before any condition has been added.
pub struct Start;

pub struct Negated<A> {
    pub inner: A,
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Start {}
    impl<P, G> Sealed for super::PendingOr<P, G> {}
    impl<P, G> Sealed for super::PendingAnd<P, G> {}
    impl<S: Sealed> Sealed for super::Negated<S> {}
}

/// A point in a builder chain that the next condition attaches to.
pub trait Step: sealed::Sealed + Sized {
    type Out<E>;

    fn then<E>(self, expr: E) -> Self::Out<E>;
}

impl Step for Start {
    type Out<E> = Query<NoAlternatives, E>;

    fn then<E>(self, expr: E) -> Self::Out<E> {
        Query {
            p: NoAlternatives,
            g: expr,
        }
    }
}

impl<P, G> Step for PendingOr<P, G> {
    type Out<E> = Query<Or<P, G>, E>;

    fn then<E>(self, expr: E) -> Self::Out<E> {
        Query {
            p: Or {
                left: self.resulting_p.p,
                right: self.resulting_p.g,
            },
            g: expr,
        }
    }
}

impl<P, G> Step for PendingAnd<P, G> {
    type Out<E> = Query<P, And<G, E>>;

    fn then<E>(self, expr: E) -> Self::Out<E> {
        Query {
            p: self.resulting_p.p,
            g: And {
                left: self.resulting_p.g,
                right: expr,
            },
        }
    }
}

impl<S: Step> Step for Negated<S> {
    type Out<E> = S::Out<Not<E>>;

    fn then<E>(self, expr: E) -> Self::Out<E> {
        self.inner.then(Not { inner: expr })
    }
}

/// Generates every condition as a method on each chain step, and as a chain head on [`Is`].
///
/// Generic names must not clash with the step impls' own (`P`, `G`, `S`).
///
/// Each entry reads `fn name[generics](args) -> ExprType => construction;`.
macro_rules! conditions {
    (@step [$($head:tt)*] $(
        $(#[$doc:meta])*
        fn $name:ident [$($gen:tt)*] ($($arg:ident: $argty:ty),*) -> $out:ty => $build:expr;
    )*) => {
        $($head)* {
            $(
                $(#[$doc])*
                pub fn $name<$($gen)*>(self, $($arg: $argty),*) -> <Self as Step>::Out<$out> {
                    self.then($build)
                }
            )*
        }
    };
    (@is $(
        $(#[$doc:meta])*
        fn $name:ident [$($gen:tt)*] ($($arg:ident: $argty:ty),*) -> $out:ty => $build:expr;
    )*) => {
        impl Is {
            $(
                $(#[$doc])*
                pub fn $name<$($gen)*>($($arg: $argty),*) -> Query<NoAlternatives, $out> {
                    Start.then($build)
                }
            )*
        }
    };
    ($($spec:tt)*) => {
        conditions!(@step [impl Start] $($spec)*);
        conditions!(@step [impl<P, G> PendingOr<P, G>] $($spec)*);
        conditions!(@step [impl<P, G> PendingAnd<P, G>] $($spec)*);
        conditions!(@step [impl<S: Step> Negated<S>] $($spec)*);
        conditions!(@is $($spec)*);
    };
}

conditions! {
    /// `None`.
    fn none[]() -> IsNone => IsNone;

    /// Equal to `value`.
    fn equal_to[T](value: T) -> Equal<T> => Equal { value };

    /// Strictly greater than `value`.
    fn greater_than[T](value: T) -> GreaterThan<T> => GreaterThan { value };

    /// Strictly less than `value`.
    fn less_than[T](value: T) -> LessThan<T> => LessThan { value };

    /// Greater than or equal to `value`.
    fn at_least[T](value: T) -> AtLeast<T> => AtLeast { value };

    /// Less than or equal to `value`.
    fn at_most[T](value: T) -> AtMost<T> => AtMost { value };

    /// Within `range`, e.g. `1..=5` or `..10`.
    fn in_range[R](range: R) -> InRange<R> => InRange { range };

    /// A string, slice or collection of length zero.
    fn empty[]() -> IsEmpty => IsEmpty;

    /// A length matching `expr`, e.g. `Is::length(Is::at_most(8))`.
    fn length[E](expr: E) -> HasLength<E> => HasLength { expr };

    /// A collection with some item equal to `value`.
    fn contains[U](value: U) -> Contains<U> => Contains { value };

    /// A collection where at least one item matches `expr`.
    fn any[E](expr: E) -> AnyItem<E> => AnyItem { expr };

    /// A collection where every item matches `expr`.
    fn all[E](expr: E) -> AllItems<E> => AllItems { expr };

    /// A string starting with `pattern`.
    fn starts_with[Pat](pattern: Pat) -> StartsWith<Pat> => StartsWith { pattern };

    /// A string ending with `pattern`.
    fn ends_with[Pat](pattern: Pat) -> EndsWith<Pat> => EndsWith { pattern };

    /// A string containing `pattern` as a substring.
    fn contains_str[Pat](pattern: Pat) -> ContainsStr<Pat> => ContainsStr { pattern };

    /// A value for which `pred` returns `true`, described as `description`.
    ///
    /// Annotate the closure's argument type, e.g. `|x: &i32| x % 2 == 0`.
    fn satisfies[D, T: ?Sized, F: Fn(&T) -> bool](description: D, pred: F)
        -> Satisfies<D, F> => Satisfies { description, pred };

    /// A value whose part borrowed by `get` matches `expr`, reported as `name`.
    ///
    /// Annotate the closure's argument type, e.g. `|u: &User| &u.age`.
    fn field[N, T: ?Sized, U: ?Sized, F: Fn(&T) -> &U, E](name: N, get: F, expr: E)
        -> Field<N, F, E> => Field { name, get, expr };

    /// A value whose part computed by `get` matches `expr`, reported as `name`.
    ///
    /// Annotate the closure's argument type, e.g. `|s: &String| s.chars().count()`.
    fn property[N, T: ?Sized, U, F: Fn(&T) -> U, E](name: N, get: F, expr: E)
        -> Property<N, F, E> => Property { name, get, expr };

    /// `expr`, described as `name` in place of its full expansion.
    fn named[N, E](name: N, expr: E) -> Named<N, E> => Named { name, expr };

    /// Any custom expression.
    fn matches[E](expr: E) -> E => expr;
}

impl Start {
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<P, G> PendingOr<P, G> {
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<P, G> PendingAnd<P, G> {
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<S: Step> Negated<S> {
    /// Double negation cancels out.
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> S {
        self.inner
    }
}

impl Is {
    #[allow(clippy::should_implement_trait)]
    pub fn not() -> Negated<Start> {
        Negated { inner: Start }
    }
}
