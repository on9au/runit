//! Verifies that building, checking, asserting and explaining never touch the heap.
//!
//! Installs a counting global allocator, so it lives in its own test binary. Counts are
//! per-thread, so concurrently running tests don't interfere.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::fmt::{self, Formatter, Write};

use runit::{And, Assert, Equal, Explanation, Expr, ExprExt, Is, Not, Or};

struct Counting;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record();
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn record() {
    let _ = TRACKING.try_with(|t| {
        if t.get() {
            let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
        }
    });
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Runs `f` and returns its result along with how many allocations it made on this thread.
fn count_allocs<R>(f: impl FnOnce() -> R) -> (R, usize) {
    ALLOCS.with(|a| a.set(0));
    TRACKING.with(|t| t.set(true));
    let r = f();
    TRACKING.with(|t| t.set(false));
    (r, ALLOCS.with(|a| a.get()))
}

/// A fixed-capacity, stack-allocated string buffer.
struct StackBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> StackBuf<N> {
    fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len]).unwrap()
    }
}

impl<const N: usize> Write for StackBuf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[test]
fn allocator_is_actually_counting() {
    let (_, n) = count_allocs(|| Box::new(1));
    assert_eq!(n, 1);
}

#[test]
fn building_expressions_does_not_allocate() {
    let (_, n) = count_allocs(|| {
        let e = Is::equal_to(1)
            .or()
            .not()
            .equal_to(2)
            .and()
            .not()
            .equal_to(3)
            .or()
            .equal_to(10);
        std::hint::black_box(&e);
        let o = Is::none().or().not().not().equal_to(Some(1));
        std::hint::black_box(&o);
    });
    assert_eq!(n, 0);
}

#[test]
fn checking_does_not_allocate() {
    let e = Is::equal_to(1)
        .or()
        .not()
        .equal_to(2)
        .and()
        .not()
        .equal_to(3)
        .or()
        .equal_to(10);
    let (results, n) = count_allocs(|| {
        let mut results = [false; 21];
        for (i, x) in (-10..=10).enumerate() {
            results[i] = e.check(&x);
        }
        results
    });
    assert_eq!(n, 0);
    assert!(results.iter().any(|&r| r) && results.iter().any(|&r| !r));
}

#[test]
fn checking_options_and_strings_does_not_allocate() {
    let opt = Is::none().or().equal_to(Some(5));
    let s = Is::equal_to("abc").or().not().equal_to("xyz");
    let (_, n) = count_allocs(|| {
        std::hint::black_box(opt.check(&None));
        std::hint::black_box(opt.check(&Some(6)));
        std::hint::black_box(s.check(&"abc"));
        std::hint::black_box(s.check(&"xyz"));
    });
    assert_eq!(n, 0);
}

#[test]
fn passing_assert_does_not_allocate() {
    let (_, n) = count_allocs(|| {
        Assert::that(&5, Is::equal_to(5));
        Assert::that(&None::<u8>, Is::none());
        Assert::that(&7, Is::not().equal_to(1).and().not().equal_to(2));
        Assert::that(&3, Is::equal_to(1).or().equal_to(2).or().equal_to(3));
    });
    assert_eq!(n, 0);
}

#[test]
fn explaining_into_a_stack_buffer_does_not_allocate() {
    let e = Is::equal_to(1).or().equal_to(2).or().not().equal_to(0);
    let mut buf = StackBuf::<256>::new();
    let (res, n) = count_allocs(|| write!(buf, "{}", e.explanation(&0)));
    assert_eq!(n, 0);
    res.unwrap();
    assert_eq!(
        buf.as_str(),
        "Expected either:\n  - Expected 1, got 0\n  - Expected 2, got 0\n  - Expected not equal to 0, but it was"
    );
}

#[test]
fn explaining_nested_indented_lists_does_not_allocate() {
    let e = Or {
        left: And {
            left: Or {
                left: Equal { value: 1 },
                right: Equal { value: 2 },
            },
            right: Not {
                inner: Equal { value: 9 },
            },
        },
        right: Equal { value: 3 },
    };
    let mut buf = StackBuf::<256>::new();
    let (res, n) = count_allocs(|| write!(buf, "{}", e.explanation(&0)));
    assert_eq!(n, 0);
    res.unwrap();
    assert_eq!(
        buf.as_str(),
        "Expected either:\n  - Expected either:\n      - Expected 1, got 0\n      - Expected 2, got 0\n  - Expected 3, got 0"
    );
}

#[test]
fn describing_into_a_stack_buffer_does_not_allocate() {
    let e = Is::not()
        .matches(Is::equal_to(1).or().equal_to(2))
        .and()
        .not()
        .equal_to(3)
        .or()
        .equal_to(4);
    let mut buf = StackBuf::<256>::new();
    let (res, n) = count_allocs(|| write!(buf, "{}", e.description()));
    assert_eq!(n, 0);
    res.unwrap();
    assert_eq!(
        buf.as_str(),
        "not (equal to 1 or equal to 2) and not equal to 3 or equal to 4"
    );
}

#[test]
fn explanation_reports_errors_from_a_full_buffer() {
    let e = Is::equal_to(123456);
    let mut buf = StackBuf::<8>::new();
    assert!(write!(buf, "{}", e.explanation(&0)).is_err());
}

#[test]
fn custom_expr_without_allocations() {
    struct Even;
    impl Expr<i32> for Even {
        fn check(&self, actual: &i32) -> bool {
            actual % 2 == 0
        }
        fn describe(&self, f: &mut Formatter<'_>) -> fmt::Result {
            f.write_str("even")
        }
    }
    let e = Is::matches(Even).or().equal_to(7);
    let mut buf = StackBuf::<128>::new();
    let (res, n) = count_allocs(|| {
        assert!(e.check(&4));
        assert!(!e.check(&5));
        write!(buf, "{}", Explanation::new(&e, &5))
    });
    assert_eq!(n, 0);
    res.unwrap();
    assert_eq!(
        buf.as_str(),
        "Expected either:\n  - Expected even\n  - Expected 7, got 5"
    );
}

#[test]
fn new_conditions_check_and_explain_without_allocating() {
    let words = ["alpha", "beta", "gamma"];
    let e = Is::all(Is::length(Is::in_range(4..=5)).and().not().starts_with("x"))
        .and()
        .contains("beta")
        .and()
        .field("first", |w: &[&str; 3]| &w[0], Is::ends_with("a"))
        .and()
        .property("count", |w: &[&str; 3]| w.len(), Is::greater_than(2))
        .named("word list")
        .and()
        .satisfies("sorted", |w: &[&str; 3]| w.is_sorted());
    let mut buf = StackBuf::<256>::new();
    let (res, n) = count_allocs(|| {
        assert!(e.validate(&words).is_ok());
        let bad = ["alpha", "beta", "zeta-long"];
        let err = e.validate(&bad).unwrap_err();
        write!(buf, "{err}")
    });
    assert_eq!(n, 0);
    res.unwrap();
    assert_eq!(
        buf.as_str(),
        "word list: Item at index 2: Expected length in range 4..=5, got 9"
    );
}
