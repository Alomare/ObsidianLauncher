//! Timings of the slow steps, written to stderr when `OBSIDIAN_TIMING` is set: `let _t = timing::span("name");`.

use std::time::Instant;

/// The variable that turns the timings on.
pub const VAR: &str = "OBSIDIAN_TIMING";

fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os(VAR).is_some())
}

/// Writes the time from now to the drop, with the name.
pub struct Span {
    name: Option<String>,
    start: Instant,
}

pub fn span(name: impl Into<String>) -> Span {
    Span { name: on().then(|| name.into()), start: Instant::now() }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(name) = &self.name {
            eprintln!("[timing] {name}: {:.1} ms", self.start.elapsed().as_secs_f64() * 1000.0);
        }
    }
}
