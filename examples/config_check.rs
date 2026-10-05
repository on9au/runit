//! A config validator built from runit expressions.
//!
//! Parses a small `key = value` config with `[sections]`, then checks it against a list of
//! named rules. Every rule is checked, so all problems are reported at once rather than only
//! the first.
//!
//! Run with `cargo run --example config_check -- examples/server.conf`.

use std::fmt;
use std::process::ExitCode;

use runit::{Expr, Is, Named};

#[derive(Default)]
struct Config {
    host: String,
    port: u16,
    workers: u32,
    log_level: String,
    timeout_ms: u64,
    max_body_kb: u32,
    allowed_origins: Vec<String>,
    tls: Tls,
}

#[derive(Default)]
struct Tls {
    enabled: bool,
    cert: String,
    key: String,
}

// ------------------------------------------------------------------ rules

/// A rule keeps its name beside the expression, so a report can show each separately.
type Rule = Named<&'static str, Box<dyn Expr<Config>>>;

fn rule(name: &'static str, expr: impl Expr<Config> + 'static) -> Rule {
    Named {
        name,
        expr: Box::new(expr),
    }
}

fn rules() -> Vec<Rule> {
    vec![
        rule(
            "host is a bare hostname",
            Is::field(
                "host",
                |c: &Config| &c.host,
                Is::not()
                    .empty()
                    .and()
                    .not()
                    .contains_str(" ")
                    .and()
                    .not()
                    .contains_str("://"),
            ),
        ),
        rule(
            "port is 80, 443 or unprivileged",
            Is::field(
                "port",
                |c: &Config| &c.port,
                Is::one_of([80, 443]).or().in_range(1024..=49151),
            ),
        ),
        rule(
            "worker count is sane",
            Is::field("workers", |c: &Config| &c.workers, Is::in_range(1..=64)),
        ),
        rule(
            "log level is known",
            Is::field(
                "log_level",
                |c: &Config| &c.log_level,
                Is::one_of(["error", "warn", "info", "debug", "trace"]),
            ),
        ),
        rule(
            "timeout is between 100ms and 60s",
            Is::field(
                "timeout_ms",
                |c: &Config| &c.timeout_ms,
                Is::in_range(100..=60_000),
            ),
        ),
        rule(
            "request body limit is at most 10 MiB",
            Is::field(
                "max_body_kb",
                |c: &Config| &c.max_body_kb,
                Is::at_most(10 * 1024),
            ),
        ),
        rule(
            "CORS origins are set and all HTTPS",
            Is::field(
                "allowed_origins",
                |c: &Config| &c.allowed_origins,
                Is::not().empty().and().all(Is::starts_with("https://")),
            ),
        ),
        // Only checked when TLS is on: "disabled, or both files are PEM".
        rule(
            "TLS files are PEM",
            Is::field(
                "tls.enabled",
                |c: &Config| &c.tls.enabled,
                Is::equal_to(false),
            )
            .or()
            .field("tls.cert", |c: &Config| &c.tls.cert, Is::ends_with(".pem"))
            .and()
            .field("tls.key", |c: &Config| &c.tls.key, Is::ends_with(".pem")),
        ),
        rule(
            "TLS on whenever the port is 443",
            Is::field("port", |c: &Config| &c.port, Is::not().equal_to(443))
                .or()
                .field(
                    "tls.enabled",
                    |c: &Config| &c.tls.enabled,
                    Is::equal_to(true),
                ),
        ),
    ]
}

// ------------------------------------------------------------------ parsing

fn parse(text: &str) -> Result<Config, Vec<String>> {
    let mut config = Config::default();
    let mut errors = Vec::new();
    let mut section = String::new();

    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = format!("{}.", name.trim());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            errors.push(format!("line {}: expected `key = value`", n + 1));
            continue;
        };
        let key = format!("{section}{}", key.trim());
        let value = value.trim();

        let result = match key.as_str() {
            "host" => set_string(&mut config.host, value),
            "port" => set_parsed(&mut config.port, value),
            "workers" => set_parsed(&mut config.workers, value),
            "log_level" => set_string(&mut config.log_level, value),
            "timeout_ms" => set_parsed(&mut config.timeout_ms, value),
            "max_body_kb" => set_parsed(&mut config.max_body_kb, value),
            "allowed_origins" => {
                config.allowed_origins = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                Ok(())
            }
            "tls.enabled" => set_parsed(&mut config.tls.enabled, value),
            "tls.cert" => set_string(&mut config.tls.cert, value),
            "tls.key" => set_string(&mut config.tls.key, value),
            _ => Err("unknown key".to_string()),
        };
        if let Err(e) = result {
            errors.push(format!("line {}: {key}: {e}", n + 1));
        }
    }

    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors)
    }
}

fn set_string(slot: &mut String, value: &str) -> Result<(), String> {
    *slot = value.to_string();
    Ok(())
}

fn set_parsed<T: std::str::FromStr>(slot: &mut T, value: &str) -> Result<(), String>
where
    T::Err: fmt::Display,
{
    *slot = value
        .parse()
        .map_err(|e| format!("invalid value {value:?} ({e})"))?;
    Ok(())
}

// ------------------------------------------------------------------ main

fn main() -> ExitCode {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/server.conf".to_string());
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let config = match parse(&text) {
        Ok(config) => config,
        Err(errors) => {
            for e in errors {
                eprintln!("parse error: {e}");
            }
            return ExitCode::from(2);
        }
    };

    println!("checking {path}\n");
    let rules = rules();
    let mut failed = 0;
    for rule in &rules {
        match rule.expr.validate(&config) {
            Ok(()) => println!("  \u{2713} {}", rule.name),
            Err(why) => {
                failed += 1;
                let why = why.to_string().replace('\n', "\n      ");
                println!("  \u{2717} {}\n      {why}", rule.name);
            }
        }
    }

    println!();
    if failed == 0 {
        println!("all {} rules passed", rules.len());
        ExitCode::SUCCESS
    } else {
        println!("{failed} of {} rules failed", rules.len());
        ExitCode::FAILURE
    }
}
