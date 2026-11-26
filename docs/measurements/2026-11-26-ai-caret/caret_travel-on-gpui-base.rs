//! How the AI caret travels. It takes off from the writer's caret when it
//! first shows, glides when it moves to another place in the text, and stays
//! on the text while the view scrolls or the lines reflow.
//!
//! The glide never animates the caret's position. It animates how much of the
//! caret's total travel has been covered, and the caret is drawn that far
//! short of where its place in the text is this frame.

/// A point in the pane, in pixels.
pub type Spot = (f32, f32);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Seen {
    /// No AI caret: the next one takes off from the writer's caret.
    #[default]
    Never,
    /// There is an AI caret, but the last frame could not draw it.
    Hidden,
    /// Drawn last frame for this place in the text; `None` is the writer's caret.
    At { pos: Option<usize>, spot: Spot },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CaretTravel {
    seen: Seen,
    travelled: Spot,
    flight: usize,
}

/// One frame of the AI caret.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretLeg {
    /// Changes whenever the caret shows anew, so its glide starts at rest.
    pub flight: usize,
    /// The sum of every move of this flight: what the glide heads for.
    pub travelled: Spot,
    /// Where the caret's place in the text is this frame.
    pub spot: Spot,
    /// The caret sits where it takes off from and moves on the next frame.
    pub taking_off: bool,
}

impl CaretLeg {
    /// Where to draw the caret once the glide has covered this much of the travel.
    pub fn drawn_at(&self, covered: Spot) -> Spot {
        (
            self.spot.0 - (self.travelled.0 - covered.0),
            self.spot.1 - (self.travelled.1 - covered.1),
        )
    }
}

impl CaretTravel {
    /// There is no AI caret any more.
    pub fn land(&mut self) {
        self.seen = Seen::Never;
    }

    /// The AI caret is out of view this frame.
    pub fn hide(&mut self) {
        if self.seen != Seen::Never {
            self.seen = Seen::Hidden;
        }
    }

    /// The caret is at `pos`, drawn at `at`. A caret that first shows starts
    /// from `takeoff`, the writer's caret, when there is one to start from.
    pub fn step(&mut self, pos: usize, at: Spot, takeoff: Option<Spot>) -> CaretLeg {
        let seen = self.seen;
        self.seen = Seen::At {
            pos: Some(pos),
            spot: at,
        };
        match seen {
            Seen::At { pos: last, spot } => {
                if last != Some(pos) {
                    self.travelled.0 += at.0 - spot.0;
                    self.travelled.1 += at.1 - spot.1;
                }
            }
            Seen::Never | Seen::Hidden => {
                self.flight = self.flight.wrapping_add(1);
                self.travelled = (0., 0.);
                if let (Seen::Never, Some(from)) = (seen, takeoff) {
                    self.seen = Seen::At {
                        pos: None,
                        spot: from,
                    };
                    return self.leg(from, true);
                }
            }
        }
        self.leg(at, false)
    }

    fn leg(&self, spot: Spot, taking_off: bool) -> CaretLeg {
        CaretLeg {
            flight: self.flight,
            travelled: self.travelled,
            spot,
            taking_off,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRITER: Spot = (40., 100.);
    const FIRST: Spot = (200., 300.);
    const SECOND: Spot = (80., 500.);

    fn halfway(leg: &CaretLeg) -> Spot {
        (leg.travelled.0 / 2., leg.travelled.1 / 2.)
    }

    #[test]
    fn a_new_caret_takes_off_from_the_writers_caret_and_glides_to_its_place() {
        let mut travel = CaretTravel::default();
        let takeoff = travel.step(10, FIRST, Some(WRITER));
        assert!(takeoff.taking_off);
        assert_eq!(takeoff.drawn_at(takeoff.travelled), WRITER);

        let flying = travel.step(10, FIRST, Some(WRITER));
        assert!(!flying.taking_off);
        assert_eq!(flying.flight, takeoff.flight);
        assert_eq!(flying.drawn_at((0., 0.)), WRITER);
        assert_eq!(flying.drawn_at(halfway(&flying)), (120., 200.));
        assert_eq!(flying.drawn_at(flying.travelled), FIRST);
    }

    #[test]
    fn a_move_to_another_place_starts_where_the_caret_was_drawn() {
        let mut travel = CaretTravel::default();
        let resting = travel.step(10, FIRST, None);
        let moved = travel.step(90, SECOND, None);
        assert_eq!(moved.drawn_at(resting.travelled), FIRST);
        assert_eq!(moved.drawn_at(moved.travelled), SECOND);
    }

    #[test]
    fn a_move_in_the_middle_of_a_glide_does_not_jump() {
        let mut travel = CaretTravel::default();
        travel.step(10, FIRST, Some(WRITER));
        let flying = travel.step(10, FIRST, None);
        let covered = halfway(&flying);
        let turned = travel.step(90, SECOND, None);
        assert_eq!(turned.drawn_at(covered), flying.drawn_at(covered));
        assert_eq!(turned.drawn_at(turned.travelled), SECOND);
    }

    #[test]
    fn scrolling_carries_the_caret_with_the_text_and_adds_no_travel() {
        let mut travel = CaretTravel::default();
        let before = travel.step(10, FIRST, None);
        let scrolled = travel.step(10, (200., 120.), None);
        assert_eq!(scrolled.travelled, before.travelled);
        assert_eq!(scrolled.drawn_at(before.travelled), (200., 120.));
    }

    #[test]
    fn with_nowhere_to_take_off_from_the_caret_shows_in_place() {
        let mut travel = CaretTravel::default();
        let leg = travel.step(10, FIRST, None);
        assert!(!leg.taking_off);
        assert_eq!(leg.drawn_at((0., 0.)), FIRST);
    }

    #[test]
    fn a_caret_back_in_view_shows_in_place_on_a_new_flight() {
        let mut travel = CaretTravel::default();
        let before = travel.step(10, FIRST, None);
        travel.hide();
        let back = travel.step(90, SECOND, Some(WRITER));
        assert!(!back.taking_off);
        assert_ne!(back.flight, before.flight);
        assert_eq!(back.drawn_at((0., 0.)), SECOND);
    }

    #[test]
    fn the_next_caret_after_one_is_gone_takes_off_again() {
        let mut travel = CaretTravel::default();
        let first = travel.step(10, FIRST, None);
        travel.land();
        travel.hide();
        let next = travel.step(90, SECOND, Some(WRITER));
        assert!(next.taking_off);
        assert_ne!(next.flight, first.flight);
        assert_eq!(next.drawn_at((0., 0.)), WRITER);
    }
}
