use core::{fmt, time::Duration};

use crate::{Animatable, Motion};

/// A trace stops after this many rows, settled or not.
const MOST_ROWS: usize = 2_000;

/// A motion written down: its value at regular times, as rows of text.
///
/// A trace is how a test, a log or a coding agent sees motion without a
/// screen. Printing it gives one row per step, with the time in milliseconds
/// and one column per channel:
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Motion, Trace, Tween};
///
/// let ms = Duration::from_millis;
/// let mut x = Motion::at(0.0);
/// x.set_target(100.0, Tween::new(ms(200), Easing::Linear), ms(0));
///
/// assert_eq!(
///     Trace::of(&x, ms(0), ms(50)).to_string(),
///     "\
/// ms           value
/// 0            0.000
/// 50          25.000
/// 100         50.000
/// 150         75.000
/// 200        100.000  settled
/// "
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    rows: Vec<Row>,
}

/// One row of a [`Trace`].
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The time of the row.
    pub at: Duration,
    /// The value of each channel at that time.
    pub values: Vec<f32>,
    /// The motion has reached its target.
    pub settled: bool,
}

impl Trace {
    /// Reads `motion` every `step`, from `from` until it settles.
    ///
    /// A `step` shorter than a millisecond counts as a millisecond, and a
    /// trace holds at most 2,000 rows.
    #[must_use]
    pub fn of<T: Animatable>(motion: &Motion<T>, from: Duration, step: Duration) -> Self {
        let step = step.max(Duration::from_millis(1));
        let mut rows = Vec::new();
        let mut at = from;
        loop {
            let sample = motion.sample(at);
            rows.push(Row {
                at,
                values: sample.value.to_channels().as_ref().to_vec(),
                settled: sample.settled,
            });
            if sample.settled || rows.len() == MOST_ROWS {
                return Self { rows };
            }
            at += step;
        }
    }

    /// The rows, in order of time.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Adds the rows of `later` after this trace's own, to write down a
    /// motion that changes course.
    pub fn extend(&mut self, later: Self) {
        self.rows.extend(later.rows);
    }
}

impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:<6}  {:>10}", "ms", "value")?;
        for row in &self.rows {
            write!(f, "{:<6}", row.at.as_millis())?;
            for value in &row.values {
                write!(f, "  {value:>10.3}")?;
            }
            writeln!(f, "{}", if row.settled { "  settled" } else { "" })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Easing, Spring, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn a_trace_ends_on_the_first_settled_row() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, Spring::SNAPPY, ms(0));
        let trace = Trace::of(&motion, ms(0), ms(16));
        let (last, earlier) = trace.rows().split_last().unwrap();
        assert!(last.settled);
        assert_eq!(last.values, [100.0]);
        assert!(earlier.iter().all(|row| !row.settled));
        assert!(earlier.len() > 10);
    }

    #[test]
    fn a_value_at_rest_is_one_settled_row() {
        let trace = Trace::of(&Motion::at((3.0, 4.0)), ms(250), ms(16));
        assert_eq!(
            trace.to_string(),
            "ms           value\n250          3.000       4.000  settled\n"
        );
    }

    #[test]
    fn a_change_of_course_reads_as_one_trace() {
        let tween = Tween::new(ms(100), Easing::Linear);
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, tween, ms(0));
        let mut trace = Trace::of(&motion, ms(0), ms(50));
        trace.rows.truncate(2);
        motion.set_target(0.0, tween, ms(50));
        trace.extend(Trace::of(&motion, ms(100), ms(50)));
        let values: Vec<f32> = trace.rows().iter().map(|row| row.values[0]).collect();
        assert_eq!(values, [0.0, 50.0, 25.0, 0.0]);
    }

    #[test]
    fn a_motion_that_never_settles_in_time_stops_the_trace() {
        let mut motion = Motion::at(0.0);
        motion.set_target(
            1.0,
            Tween::new(Duration::from_secs(3_600), Easing::Linear),
            ms(0),
        );
        assert_eq!(
            Trace::of(&motion, ms(0), Duration::ZERO).rows().len(),
            MOST_ROWS
        );
    }
}
