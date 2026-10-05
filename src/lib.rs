use std::fmt::Debug;

pub trait Expr<T: ?Sized> {
    fn eval(&self, actual: &T) -> ExprResult;

    fn describe(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
}

pub struct ExprResult {
    pub pass: bool,
    pub message: String,
}

impl ExprResult {
    pub fn pass() -> Self {
        Self {
            pass: true,
            message: String::new(),
        }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            pass: false,
            message: message.into(),
        }
    }
}

pub struct Assert;

impl Assert {
    pub fn that<T, C>(actual: &T, constraint: C)
    where
        T: ?Sized,
        C: Expr<T>,
    {
        let result = constraint.eval(actual);

        if !result.pass {
            panic!("{}", result.message);
        }
    }
}

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
    fn eval(&self, actual: &T) -> ExprResult {
        let p = self.p.eval(actual);
        if p.pass {
            return p;
        }
        let g = self.g.eval(actual);
        if g.pass {
            return g;
        }
        ExprResult::fail(format!(
            "Expected either:\n  - {}\n  - {}",
            p.message, g.message
        ))
    }
}

pub struct PendingOr<P, G> {
    resulting_p: Query<P, G>,
}
pub struct PendingAnd<P, G> {
    resulting_p: Query<P, G>,
}

impl<P, G> Negated<PendingOr<P, G>> {
    pub fn none(self) -> Query<Or<P, G>, Not<IsNone>> {
        Query {
            p: Or {
                left: self.inner.resulting_p.p,
                right: self.inner.resulting_p.g,
            },
            g: Not { inner: IsNone },
        }
    }

    pub fn equal_to<T>(self, value: T) -> Query<Or<P, G>, Not<Equal<T>>> {
        Query {
            p: Or {
                left: self.inner.resulting_p.p,
                right: self.inner.resulting_p.g,
            },
            g: Not {
                inner: Equal { value },
            },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}
impl<P, G> PendingOr<P, G> {
    pub fn none(self) -> Query<Or<P, G>, IsNone> {
        Query {
            p: Or {
                left: self.resulting_p.p,
                right: self.resulting_p.g,
            },
            g: IsNone,
        }
    }

    pub fn equal_to<T>(self, value: T) -> Query<Or<P, G>, Equal<T>> {
        Query {
            p: Or {
                left: self.resulting_p.p,
                right: self.resulting_p.g,
            },
            g: Equal { value },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

impl<P, G> Negated<PendingAnd<P, G>> {
    pub fn none(self) -> Query<P, And<G, Not<IsNone>>> {
        Query {
            p: self.inner.resulting_p.p,
            g: And {
                left: self.inner.resulting_p.g,
                right: Not { inner: IsNone },
            },
        }
    }

    pub fn equal_to<T>(self, value: T) -> Query<P, And<G, Not<Equal<T>>>> {
        Query {
            p: self.inner.resulting_p.p,
            g: And {
                left: self.inner.resulting_p.g,
                right: Not {
                    inner: Equal { value },
                },
            },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}
impl<P, G> PendingAnd<P, G> {
    pub fn none(self) -> Query<P, And<G, IsNone>> {
        Query {
            p: self.resulting_p.p,
            g: And {
                left: self.resulting_p.g,
                right: IsNone,
            },
        }
    }

    pub fn equal_to<T>(self, value: T) -> Query<P, And<G, Equal<T>>> {
        Query {
            p: self.resulting_p.p,
            g: And {
                left: self.resulting_p.g,
                right: Equal { value },
            },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

pub struct Is;

impl Is {
    pub fn none() -> Query<AlwaysFalse, IsNone> {
        Query {
            p: AlwaysFalse,
            g: IsNone,
        }
    }

    pub fn equal_to<T>(value: T) -> Query<AlwaysFalse, Equal<T>> {
        Query {
            p: AlwaysFalse,
            g: Equal { value },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not() -> Negated<Self> {
        Negated { inner: Is }
    }
}

pub struct Negated<A> {
    pub inner: A,
}

impl Negated<Is> {
    pub fn none(self) -> Query<AlwaysFalse, Not<IsNone>> {
        Query {
            p: AlwaysFalse,
            g: Not { inner: IsNone },
        }
    }

    pub fn equal_to<T>(self, value: T) -> Query<AlwaysFalse, Not<Equal<T>>> {
        Query {
            p: AlwaysFalse,
            g: Not {
                inner: Equal { value },
            },
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(self) -> Negated<Self> {
        Negated { inner: self }
    }
}

pub struct AlwaysFalse;
pub struct IsNone;
pub struct Not<A> {
    pub inner: A,
}
pub struct Equal<T> {
    pub value: T,
}
pub struct And<A, B> {
    pub left: A,
    pub right: B,
}
pub struct Or<A, B> {
    pub left: A,
    pub right: B,
}

impl<T: ?Sized> Expr<T> for AlwaysFalse {
    fn eval(&self, _actual: &T) -> ExprResult {
        ExprResult::fail("Condition will always fail.")
    }
}

impl<T> Expr<Option<T>> for IsNone {
    fn eval(&self, actual: &Option<T>) -> ExprResult {
        if actual.is_none() {
            ExprResult::pass()
        } else {
            ExprResult::fail("Expected null")
        }
    }
}

impl<T: PartialEq + Debug> Expr<T> for Equal<T> {
    fn eval(&self, actual: &T) -> ExprResult {
        if &self.value == actual {
            ExprResult::pass()
        } else {
            ExprResult::fail(format!("Expected {:?}, got {:?}", self.value, actual))
        }
    }
}

impl<A, B, T> Expr<T> for And<A, B>
where
    T: ?Sized,
    A: Expr<T>,
    B: Expr<T>,
{
    fn eval(&self, actual: &T) -> ExprResult {
        let l = self.left.eval(actual);
        if !l.pass {
            return l;
        }
        let r = self.right.eval(actual);
        if !r.pass {
            return r;
        }
        ExprResult::pass()
    }
}

impl<A, B, T> Expr<T> for Or<A, B>
where
    T: ?Sized,
    A: Expr<T>,
    B: Expr<T>,
{
    fn eval(&self, actual: &T) -> ExprResult {
        let p = self.left.eval(actual);
        if p.pass {
            return p;
        }
        let g = self.right.eval(actual);
        if g.pass {
            return g;
        }
        ExprResult::fail(format!(
            "Expected either:\n  - {}\n  - {}",
            p.message, g.message
        ))
    }
}

impl<A, T> Expr<T> for Not<A>
where
    T: ?Sized,
    A: Expr<T>,
{
    fn eval(&self, actual: &T) -> ExprResult {
        let inner_result = self.inner.eval(actual);
        if inner_result.pass {
            ExprResult::fail("Expected condition to fail, but it passed.")
        } else {
            ExprResult::pass()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null() {
        let bruh: Option<i32> = None;
        Assert::that(&bruh, Is::none());
        Assert::that(&bruh, Is::not().none());
    }
}
