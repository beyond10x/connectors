//! Wall time per step of a `connectors-build` program. A job's split is read from one place
//! in its log, the closing table, and not from the timestamps between two lines
//! (story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step).

use std::time::{Duration, Instant};

/// Where a program's step lines go. `ignored` prints its report on standard output, so its
/// step lines go to standard error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Out,
    Err,
}

/// The steps of one program run, in order. A step begins with [`Steps::begin`] and ends when
/// the next one begins, at [`Steps::finish`], or when the value is dropped: an error that
/// leaves the function drops it, so the table is printed for a failed run too and names the
/// step that did not end.
pub struct Steps {
    program: &'static str,
    stream: Stream,
    rows: Vec<(String, Duration)>,
    open: Option<(String, Instant)>,
}

impl Steps {
    pub fn new(program: &'static str, stream: Stream) -> Self {
        Self {
            program,
            stream,
            rows: Vec::new(),
            open: None,
        }
    }

    /// Ends the step before this one, printing its line, and starts `name`.
    pub fn begin(&mut self, name: &str) {
        let now = Instant::now();
        self.end(now, "");
        self.open = Some((name.to_owned(), now));
    }

    /// Ends the last step: the program reached its end without an error.
    pub fn finish(&mut self) {
        self.end(Instant::now(), "");
    }

    fn end(&mut self, now: Instant, note: &str) {
        if let Some((name, started)) = self.open.take() {
            let name = format!("{name}{note}");
            let elapsed = now.saturating_duration_since(started);
            self.say(&line(self.program, &name, elapsed));
            self.rows.push((name, elapsed));
        }
    }

    fn say(&self, text: &str) {
        match self.stream {
            Stream::Out => println!("{text}"),
            Stream::Err => eprintln!("{text}"),
        }
    }
}

impl Drop for Steps {
    fn drop(&mut self) {
        self.end(Instant::now(), NOT_ENDED);
        for row in table(self.program, &self.rows) {
            self.say(&row);
        }
    }
}

/// What the table adds to the name of a step an error ended.
const NOT_ENDED: &str = " (ended by an error)";

/// The line a step prints when it ends.
fn line(program: &str, name: &str, elapsed: Duration) -> String {
    format!("{program}: step ran {:.1} s: {name}", elapsed.as_secs_f64())
}

/// The closing table: a heading, one row per step in the order they ran, and their sum.
fn table(program: &str, rows: &[(String, Duration)]) -> Vec<String> {
    let mut lines = vec![format!("{program}: time per step")];
    for (name, elapsed) in rows {
        lines.push(format!(
            "{program}: {:>8.1} s  {name}",
            elapsed.as_secs_f64()
        ));
    }
    let sum: Duration = rows.iter().map(|(_, elapsed)| *elapsed).sum();
    lines.push(format!(
        "{program}: {:>8.1} s  sum of {} steps",
        sum.as_secs_f64(),
        rows.len()
    ));
    lines
}

/// The names a body of `run` begins a step with, in order, read from its source text: `run`
/// of the gate builds and tests the whole workspace, so no test can call it.
#[cfg(test)]
pub fn begun(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("steps.begin(")?;
            Some(rest.strip_suffix(");")?.to_owned())
        })
        .collect()
}

/// The body of the function with this signature line, read from a source text.
#[cfg(test)]
pub fn body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(signature).expect("the signature") + signature.len();
    let end = source[start..].find("\n}\n").expect("the end of the body");
    &source[start..start + end]
}

#[cfg(test)]
mod tests {
    use super::{NOT_ENDED, Steps, Stream, line, table};
    use std::time::Duration;

    fn seconds(value: f64) -> Duration {
        Duration::from_secs_f64(value)
    }

    #[test]
    fn a_step_line_names_the_program_the_seconds_and_the_step() {
        assert_eq!(
            line("gate", "build of the workspace", seconds(449.24)),
            "gate: step ran 449.2 s: build of the workspace"
        );
    }

    #[test]
    fn the_closing_table_lists_every_step_in_order_and_their_sum() {
        let rows = vec![
            ("build of the workspace".to_owned(), seconds(449.2)),
            ("tests".to_owned(), seconds(842.2)),
            ("planning validation".to_owned(), seconds(0.04)),
        ];
        assert_eq!(
            table("gate", &rows),
            [
                "gate: time per step",
                "gate:    449.2 s  build of the workspace",
                "gate:    842.2 s  tests",
                "gate:      0.0 s  planning validation",
                "gate:   1291.4 s  sum of 3 steps",
            ]
        );
    }

    #[test]
    fn a_run_without_a_step_prints_a_table_with_a_sum_of_zero() {
        assert_eq!(
            table("ignored", &[]),
            [
                "ignored: time per step",
                "ignored:      0.0 s  sum of 0 steps"
            ]
        );
    }

    #[test]
    fn a_step_ends_when_the_next_begins_and_the_last_at_finish() {
        let mut steps = Steps::new("gate", Stream::Err);
        steps.begin("one");
        assert!(steps.rows.is_empty());
        steps.begin("two");
        assert_eq!(steps.rows.len(), 1);
        assert_eq!(steps.rows[0].0, "one");
        steps.finish();
        let names: Vec<&str> = steps.rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["one", "two"]);
        assert!(steps.open.is_none());
    }

    #[test]
    fn a_step_an_error_leaves_open_is_in_the_table_as_the_one_that_did_not_end() {
        let mut steps = Steps::new("gate", Stream::Err);
        steps.begin("one");
        steps.begin("two");
        // What `Drop` does first, before it prints the table.
        steps.end(std::time::Instant::now(), NOT_ENDED);
        let names: Vec<&str> = steps.rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["one", "two (ended by an error)"]);
    }

    #[test]
    fn a_finished_run_names_no_step_as_ended_by_an_error() {
        let mut steps = Steps::new("gate", Stream::Err);
        steps.begin("one");
        steps.finish();
        steps.end(std::time::Instant::now(), NOT_ENDED);
        let names: Vec<&str> = steps.rows.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["one"]);
    }

    #[test]
    fn the_names_a_body_begins_are_read_in_order() {
        let body = "    let x = 1;\n    steps.begin(\"one\");\n    if y {\n        steps.begin(&format!(\"two {x}\"));\n    }\n";
        assert_eq!(super::begun(body), ["\"one\"", "&format!(\"two {x}\")"]);
    }
}
