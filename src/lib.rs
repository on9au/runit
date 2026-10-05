use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;

pub trait Expr<T: ?Sized> {
    fn eval(&self, actual: &T) -> ExprResult;

    fn describe(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }

    /// Evaluates `self` as a list of alternatives, pushing the message of each failed
    /// alternative into `failures`. Returns the passing result as soon as one passes.
    ///
    /// Only OR-like expressions need to override this as the default treats `self` as a
    /// single alternative. It lets nested ORs report one flat list of alternatives.
    fn eval_alternatives(&self, actual: &T, failures: &mut Vec<String>) -> Option<ExprResult> {
        let result = self.eval(actual);
        if result.pass {
            Some(result)
        } else {
            failures.push(result.message);
            None
        }
    }
}

macro_rules! forward_expr {
    ($($ptr:ty),*) => {$(
        impl<T: ?Sized, E: Expr<T> + ?Sized> Expr<T> for $ptr {
            fn eval(&self, actual: &T) -> ExprResult {
                (**self).eval(actual)
            }

            fn describe(&self) -> String {
                (**self).describe()
            }

            fn eval_alternatives(
                &self,
                actual: &T,
                failures: &mut Vec<String>,
            ) -> Option<ExprResult> {
                (**self).eval_alternatives(actual, failures)
            }
        }
    )*};
}

forward_expr!(&E, Box<E>, Rc<E>, Arc<E>);

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
    #[track_caller]
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
        let mut failures = Vec::new();
        self.eval_alternatives(actual, &mut failures)
            .unwrap_or_else(|| ExprResult::fail(either_message(failures)))
    }

    fn describe(&self) -> String {
        join(" or ", self.p.describe(), self.g.describe())
    }

    fn eval_alternatives(&self, actual: &T, failures: &mut Vec<String>) -> Option<ExprResult> {
        if let Some(r) = self.p.eval_alternatives(actual, failures) {
            return Some(r);
        }
        self.g.eval_alternatives(actual, failures)
    }
}

pub struct PendingOr<P, G> {
    resulting_p: Query<P, G>,
}
pub struct PendingAnd<P, G> {
    resulting_p: Query<P, G>,
}

pub struct Is;

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

pub struct AlwaysFalse;
pub struct NoAlternatives;
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

    fn describe(&self) -> String {
        "always false".to_string()
    }
}

impl<T: ?Sized> Expr<T> for NoAlternatives {
    fn eval(&self, _actual: &T) -> ExprResult {
        ExprResult::fail("No alternatives to match.")
    }

    fn describe(&self) -> String {
        String::new()
    }

    fn eval_alternatives(&self, _actual: &T, _failures: &mut Vec<String>) -> Option<ExprResult> {
        None
    }
}

impl<T> Expr<Option<T>> for IsNone {
    fn eval(&self, actual: &Option<T>) -> ExprResult {
        if actual.is_none() {
            ExprResult::pass()
        } else {
            ExprResult::fail("Expected None, got Some(_)")
        }
    }

    fn describe(&self) -> String {
        "None".to_string()
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

    fn describe(&self) -> String {
        format!("equal to {:?}", self.value)
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

    fn describe(&self) -> String {
        format!(
            "{} and {}",
            group(self.left.describe(), &[" or "]),
            group(self.right.describe(), &[" or "])
        )
    }
}

impl<A, B, T> Expr<T> for Or<A, B>
where
    T: ?Sized,
    A: Expr<T>,
    B: Expr<T>,
{
    fn eval(&self, actual: &T) -> ExprResult {
        let mut failures = Vec::new();
        self.eval_alternatives(actual, &mut failures)
            .unwrap_or_else(|| ExprResult::fail(either_message(failures)))
    }

    fn describe(&self) -> String {
        join(" or ", self.left.describe(), self.right.describe())
    }

    fn eval_alternatives(&self, actual: &T, failures: &mut Vec<String>) -> Option<ExprResult> {
        if let Some(r) = self.left.eval_alternatives(actual, failures) {
            return Some(r);
        }
        self.right.eval_alternatives(actual, failures)
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
            ExprResult::fail(format!("Expected {}, but it was", self.describe()))
        } else {
            ExprResult::pass()
        }
    }

    fn describe(&self) -> String {
        format!("not {}", group(self.inner.describe(), &[" or ", " and "]))
    }
}

/// Formats failed alternatives as a single message, flat and indented.
fn either_message(failures: Vec<String>) -> String {
    match failures.len() {
        0 => "No alternatives to match.".to_string(),
        1 => failures.into_iter().next().unwrap(),
        _ => {
            let mut out = String::from("Expected either:");
            for failure in &failures {
                out.push_str("\n  - ");
                out.push_str(&failure.replace('\n', "\n    "));
            }
            out
        }
    }
}

/// Joins two descriptions, skipping empty ones.
fn join(sep: &str, a: String, b: String) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b,
        (_, true) => a,
        _ => format!("{a}{sep}{b}"),
    }
}

/// Parenthesizes a description containing any of `ops`, to keep precedence readable.
fn group(desc: String, ops: &[&str]) -> String {
    if ops.iter().any(|op| desc.contains(op)) {
        format!("({desc})")
    } else {
        desc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null() {
        let bruh: Option<i32> = None;
        Assert::that(&bruh, Is::none());
    }
}
