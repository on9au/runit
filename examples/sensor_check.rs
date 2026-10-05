//! An embedded-style sensor sanity checker, simulated on desktop.
//!
//! The `firmware` module uses only `core` and runit: no `Box`, `Vec`, `String` or
//! `format!`. Rules live on the stack, failure explanations are written into a fixed-size
//! buffer standing in for a UART, and a counting allocator proves the firmware never
//! touches the heap.
//!
//! Run with `cargo run --example sensor_check`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

mod firmware {
    use core::fmt::{self, Write};

    use runit::{Description, Expr, Is, Named};

    #[derive(Debug, Clone, Copy)]
    pub struct Reading {
        pub temp_c: f32,
        pub humidity_pct: f32,
        pub supply_mv: u16,
        /// Accelerometer X/Y/Z in milli-g.
        pub accel_mg: [i16; 3],
    }

    /// What the rules see: this tick's reading and the last good one.
    pub struct Frame {
        pub now: Reading,
        pub prev: Reading,
    }

    fn abs(x: f32) -> f32 {
        if x < 0.0 { -x } else { x }
    }

    /// The rule set, built on the stack once at boot.
    pub struct Rules<A, B, C, D, E> {
        temp: A,
        humidity: B,
        supply: C,
        accel: D,
        slew: E,
    }

    pub fn rules() -> Rules<
        impl Expr<Frame>,
        impl Expr<Frame>,
        impl Expr<Frame>,
        impl Expr<Frame>,
        impl Expr<Frame>,
    > {
        Rules {
            temp: Is::field(
                "temperature",
                |f: &Frame| &f.now.temp_c,
                Is::in_range(-40.0..=85.0),
            ),
            humidity: Is::field(
                "humidity",
                |f: &Frame| &f.now.humidity_pct,
                Is::in_range(0.0..=100.0),
            ),
            supply: Is::field(
                "supply (mV)",
                |f: &Frame| &f.now.supply_mv,
                Is::in_range(3000..=3600),
            ),
            accel: Is::field(
                "accel (mg)",
                |f: &Frame| &f.now.accel_mg,
                Is::all(Is::in_range(-2000..=2000))
                    .and()
                    .not()
                    .all(Is::equal_to(0))
                    .named("in range and not stuck at zero"),
            ),
            slew: Is::property(
                "temperature change (C/tick)",
                |f: &Frame| abs(f.now.temp_c - f.prev.temp_c),
                Is::at_most(5.0),
            ),
        }
    }

    impl<A, B, C, D, E> Rules<A, B, C, D, E>
    where
        A: Expr<Frame>,
        B: Expr<Frame>,
        C: Expr<Frame>,
        D: Expr<Frame>,
        E: Expr<Frame>,
    {
        /// The rules as a fixed-size array of named trait objects, borrowed from `self`.
        fn named(&self) -> [Named<&'static str, &dyn Expr<Frame>>; 5] {
            [
                Named {
                    name: "TEMP",
                    expr: &self.temp,
                },
                Named {
                    name: "HUM",
                    expr: &self.humidity,
                },
                Named {
                    name: "PWR",
                    expr: &self.supply,
                },
                Named {
                    name: "ACCEL",
                    expr: &self.accel,
                },
                Named {
                    name: "SLEW",
                    expr: &self.slew,
                },
            ]
        }

        /// Writes the boot banner listing every rule.
        pub fn banner(&self, out: &mut impl Write) -> fmt::Result {
            writeln!(out, "monitoring:")?;
            for rule in self.named() {
                writeln!(out, "  {:<5} {}", rule.name, Description::new(rule.expr))?;
            }
            Ok(())
        }

        /// Checks one frame, writing a status line plus one line per fault. Returns the
        /// number of faults.
        pub fn tick(&self, n: u32, frame: &Frame, out: &mut impl Write) -> Result<u32, fmt::Error> {
            let mut faults = 0;
            for rule in self.named() {
                if !rule.expr.check(frame) {
                    faults += 1;
                }
            }
            write!(out, "tick {n:>2} temp={:>6.1}C ", frame.now.temp_c)?;
            if faults == 0 {
                return writeln!(out, "ok").map(|()| 0);
            }
            writeln!(out, "FAULT x{faults}")?;
            for rule in self.named() {
                if let Err(why) = Expr::validate(&rule.expr, frame) {
                    writeln!(out, "  [{}] {why}", rule.name)?;
                }
            }
            Ok(faults)
        }
    }

    /// A fixed-capacity output buffer standing in for a UART transmit buffer.
    pub struct Uart<const N: usize> {
        buf: [u8; N],
        len: usize,
    }

    impl<const N: usize> Uart<N> {
        pub const fn new() -> Self {
            Self {
                buf: [0; N],
                len: 0,
            }
        }

        pub fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buf[..self.len]).unwrap_or("<invalid utf-8>")
        }

        pub fn clear(&mut self) {
            self.len = 0;
        }
    }

    impl<const N: usize> Write for Uart<N> {
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
}

// ------------------------------------------------------------------ desktop harness

struct Counting;

static TRACKING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Runs `f` with allocation counting switched on.
fn on_device<R>(f: impl FnOnce() -> R) -> R {
    TRACKING.store(true, Ordering::Relaxed);
    let r = f();
    TRACKING.store(false, Ordering::Relaxed);
    r
}

/// A deterministic sensor feed with a few injected faults.
fn simulate(tick: u32) -> firmware::Reading {
    let wobble = ((tick * 7919) % 11) as f32 / 10.0 - 0.5;
    let mut r = firmware::Reading {
        temp_c: 21.0 + wobble,
        humidity_pct: 45.0 + wobble * 4.0,
        supply_mv: 3300 + (tick % 5) as u16 * 10,
        accel_mg: [12, -8, 1000],
    };
    match tick {
        4 => r.temp_c = -80.0,               // thermistor disconnected
        7 => r.supply_mv = 2850,             // brownout
        9 => r.accel_mg = [0, 0, 0],         // accelerometer stuck
        11 => r.humidity_pct = f32::NAN,     // bad I2C read
        13 => r.temp_c = 34.0,               // implausible jump
        15 => r.accel_mg = [12, 2600, 1000], // out of range spike
        _ => {}
    }
    r
}

fn main() {
    let mut uart = firmware::Uart::<512>::new();

    let rules = on_device(firmware::rules);
    on_device(|| rules.banner(&mut uart)).expect("banner fits in the UART buffer");
    println!("{}", uart.as_str());

    // Rate-of-change is measured against the last reading that passed every rule.
    let mut last_good = simulate(0);
    let mut faults = 0;
    for n in 1..=16 {
        let now = simulate(n);
        uart.clear();
        let frame = firmware::Frame {
            now,
            prev: last_good,
        };
        match on_device(|| rules.tick(n, &frame, &mut uart)) {
            Ok(0) => last_good = now,
            Ok(count) => faults += count,
            Err(_) => println!("(UART buffer overflow on tick {n})"),
        }
        print!("{}", uart.as_str());
    }

    println!(
        "\n{faults} faults over 16 ticks, heap allocations on device: {}",
        ALLOCS.load(Ordering::Relaxed)
    );
}
