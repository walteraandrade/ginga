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
    columns: Vec<&'static str>,
    rows: Vec<Row>,
}

/// What a [`Trace`] can write down: a [`Motion`], a
/// [`LookMotion`](crate::LookMotion) or [`Keyframes`](crate::Keyframes). It is
/// sealed: those are the only ones.
///
/// ```compile_fail
/// use std::time::Duration;
/// use ginga::{Row, Traceable};
///
/// struct Mine;
///
/// impl Traceable for Mine {
///     fn row(&self, at: Duration) -> Row {
///         Row { at, values: Vec::new(), settled: true }
///     }
/// }
/// ```
pub trait Traceable: sealed::Sealed {
    /// The row for the time `at`.
    fn row(&self, at: Duration) -> Row;

    /// A name for each column of a row. With none, the trace's header says
    /// `value` over all of them.
    fn columns(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

pub(crate) mod sealed {
    /// Keeps [`Traceable`](super::Traceable) to the crate's own types.
    pub trait Sealed {}
}

impl<T: Animatable> sealed::Sealed for Motion<T> {}

impl<T: Animatable> Traceable for Motion<T> {
    fn row(&self, at: Duration) -> Row {
        let sample = self.sample(at);
        Row {
            at,
            values: sample.value.to_channels().as_ref().to_vec(),
            settled: sample.settled,
        }
    }
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
    pub fn of(motion: &impl Traceable, from: Duration, step: Duration) -> Self {
        let step = step.max(Duration::from_millis(1));
        let mut rows = Vec::new();
        let mut at = from;
        loop {
            let row = motion.row(at);
            let settled = row.settled;
            rows.push(row);
            if settled || rows.len() == MOST_ROWS {
                return Self {
                    columns: motion.columns(),
                    rows,
                };
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
    ///
    /// # Panics
    ///
    /// When `later` names other columns than this trace, as a look motion does
    /// after a change of the values its look names: its rows would print under
    /// the wrong names. Print such traces one after another instead.
    pub fn extend(&mut self, later: Self) {
        assert_eq!(
            self.columns, later.columns,
            "the later trace names other columns; print the two traces one after another"
        );
        self.rows.extend(later.rows);
    }
}

impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<6}", "ms")?;
        if self.columns.is_empty() {
            // A motion's values have no names; a look that names none has no
            // values to put a header over.
            if self.rows.iter().any(|row| !row.values.is_empty()) {
                write!(f, "  {:>10}", "value")?;
            }
            writeln!(f)?;
        } else {
            for column in &self.columns {
                write!(f, "  {column:>10}")?;
            }
            writeln!(f)?;
        }
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
    #[should_panic(expected = "the later trace names other columns")]
    fn a_trace_refuses_to_extend_with_rows_under_other_names() {
        let width = crate::LookMotion::at(crate::Look::new().width(1.0));
        let both = crate::LookMotion::at(crate::Look::new().opacity(1.0).width(1.0));
        let mut trace = Trace::of(&width, ms(0), ms(16));
        trace.extend(Trace::of(&both, ms(16), ms(16)));
    }

    #[test]
    fn a_look_that_names_nothing_prints_no_value_header() {
        let nothing = crate::LookMotion::at(crate::Look::new());
        assert_eq!(
            Trace::of(&nothing, ms(0), ms(16)).to_string(),
            "ms    \n0       settled\n"
        );
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
            Tween::new(Duration::from_secs(10), Easing::Linear),
            ms(0),
        );
        assert_eq!(
            Trace::of(&motion, ms(0), Duration::ZERO).rows().len(),
            MOST_ROWS
        );
    }
}
