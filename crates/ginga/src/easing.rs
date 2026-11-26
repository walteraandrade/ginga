use core::f32::consts::PI;

/// The shape of a [`Tween`](crate::Tween): how progress in time becomes progress in value.
#[derive(Debug, Clone, Copy)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// Starts slow and ends fast. For things that leave.
    In,
    /// Starts fast and ends slow. For things that arrive.
    Out,
    /// Slow at both ends. For things that move from place to place.
    InOut,
    /// A sharper [`Easing::Out`].
    OutQuart,
    /// Covers most of the distance at once, then takes long to land.
    OutExpo,
    /// A gentle [`Easing::InOut`], for loops that breathe.
    InOutSine,
    /// The CSS `cubic-bezier(x1, y1, x2, y2)`.
    Bezier(f32, f32, f32, f32),
    /// Your own curve. It must return `0.0` at `0.0` and `1.0` at `1.0`.
    Custom(fn(f32) -> f32),
}

impl Easing {
    /// The progress in value at `progress` in time. Time is clamped to `0.0..=1.0`.
    ///
    /// ```
    /// use ginga::Easing;
    ///
    /// assert_eq!(Easing::Linear.at(0.25), 0.25);
    /// assert!(Easing::OutExpo.at(0.25) > 0.8);
    /// ```
    #[must_use]
    pub fn at(self, progress: f32) -> f32 {
        let t = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::In => t * t * t,
            Self::Out => 1.0 - (1.0 - t).powi(3),
            Self::InOut if t < 0.5 => 4.0 * t * t * t,
            Self::InOut => 1.0 - (2.0 - 2.0 * t).powi(3) / 2.0,
            Self::OutQuart => 1.0 - (1.0 - t).powi(4),
            Self::OutExpo if t >= 1.0 => 1.0,
            Self::OutExpo => 1.0 - 2f32.powf(-10.0 * t),
            Self::InOutSine => 0.5 - 0.5 * (PI * t).cos(),
            Self::Bezier(x1, y1, x2, y2) => bezier(x1, y1, x2, y2, t),
            Self::Custom(curve) => curve(t),
        }
    }

    /// How fast the value moves at `progress`, in value progress per time progress.
    pub(crate) fn slope(self, progress: f32) -> f32 {
        const STEP: f32 = 1e-3;
        let low = (progress - STEP).max(0.0);
        let high = (progress + STEP).min(1.0);
        if high <= low {
            return 0.0;
        }
        (self.at(high) - self.at(low)) / (high - low)
    }
}

fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, time: f32) -> f32 {
    let (cx, cy) = (3.0 * x1, 3.0 * y1);
    let (bx, by) = (3.0 * (x2 - x1) - cx, 3.0 * (y2 - y1) - cy);
    let (ax, ay) = (1.0 - cx - bx, 1.0 - cy - by);
    let x = |s: f32| ((ax * s + bx) * s + cx) * s;
    let y = |s: f32| ((ay * s + by) * s + cy) * s;
    let slope = |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;
    let mut s = time;
    for _ in 0..6 {
        let error = x(s) - time;
        if error.abs() < 1e-6 {
            return y(s);
        }
        let slope = slope(s);
        if slope.abs() < 1e-6 {
            break;
        }
        s = (s - error / slope).clamp(0.0, 1.0);
    }
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let error = x(s) - time;
        if error.abs() < 1e-6 {
            break;
        }
        if error < 0.0 {
            low = s;
        } else {
            high = s;
        }
        s = f32::midpoint(low, high);
    }
    y(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Easing; 8] = [
        Easing::Linear,
        Easing::In,
        Easing::Out,
        Easing::InOut,
        Easing::OutQuart,
        Easing::OutExpo,
        Easing::InOutSine,
        Easing::Bezier(0.16, 1.0, 0.3, 1.0),
    ];

    #[test]
    fn every_curve_starts_at_zero_and_lands_on_one() {
        for easing in ALL {
            assert!(easing.at(0.0).abs() < 2e-3, "{easing:?} at the start");
            assert!((easing.at(1.0) - 1.0).abs() < 1e-5, "{easing:?} at the end");
        }
    }

    #[test]
    fn time_outside_the_run_is_clamped() {
        for easing in ALL {
            assert_eq!(easing.at(-3.0), easing.at(0.0));
            assert_eq!(easing.at(7.0), easing.at(1.0));
        }
    }

    #[test]
    fn a_linear_bezier_is_linear() {
        let line = Easing::Bezier(0.25, 0.25, 0.75, 0.75);
        for step in 0..=10 {
            let t = step as f32 / 10.0;
            assert!((line.at(t) - t).abs() < 1e-4);
        }
    }

    #[test]
    fn the_css_ease_out_expo_bezier_runs_ahead_of_time() {
        let css = Easing::Bezier(0.16, 1.0, 0.3, 1.0);
        assert!(css.at(0.25) > 0.75);
        assert!(css.at(0.5) > 0.95);
    }

    #[test]
    fn the_slope_of_a_line_is_one_and_an_arriving_curve_ends_flat() {
        assert!((Easing::Linear.slope(0.5) - 1.0).abs() < 1e-3);
        assert!(Easing::Out.slope(1.0) < 0.01);
        assert!(Easing::Out.slope(0.0) > 2.9);
    }
}
