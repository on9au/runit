use core::fmt::{self, Formatter};

use crate::combinators::{describe_or, explain_or, precedence_or};
use crate::{And, Equal, Expr, IsNone, NoAlternatives, Not, Or, Precedence};

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

macro_rules! conditions {
    () => {
        pub fn none(self) -> <Self as Step>::Out<IsNone> {
            self.then(IsNone)
        }

        pub fn equal_to<T>(self, value: T) -> <Self as Step>::Out<Equal<T>> {
            self.then(Equal { value })
        }

        pub fn matches<E>(self, expr: E) -> <Self as Step>::Out<E> {
            self.then(expr)
        }
    };
}

impl Start {
    conditions!();

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<P, G> PendingOr<P, G> {
    conditions!();

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<P, G> PendingAnd<P, G> {
    conditions!();

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<S: Step> Negated<S> {
    conditions!();

    /// Double negation cancels out.
    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> S {
        self.inner
    }
}

impl Is {
    pub fn none() -> Query<NoAlternatives, IsNone> {
        Start.then(IsNone)
    }

    pub fn equal_to<T>(value: T) -> Query<NoAlternatives, Equal<T>> {
        Start.then(Equal { value })
    }

    pub fn matches<E>(expr: E) -> Query<NoAlternatives, E> {
        Start.then(expr)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not() -> Negated<Start> {
        Negated { inner: Start }
    }
}
