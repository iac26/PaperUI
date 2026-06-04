//! Pure debounce/gesture detection for this board's physical buttons. `buttons.rs` feeds
//! raw GPIO state in; the board then maps the resulting gestures to the engine's `UiEvent`s.
//! Hardware-agnostic and deterministic (no esp-hal), so it stays host-testable.

/// Press duration (ms) at/after which a press becomes a Hold (not a Click).
pub const HOLD_MS: u32 = 600;
/// Max gap (ms) between a Click's release and the next press to form a DoubleClick.
pub const DOUBLE_MS: u32 = 250;

/// The M5StickC's physical buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonId {
    A,
    B,
    C,
}

/// A gesture produced by [`GestureState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonEvent {
    Click,
    Hold,
    DoubleClick,
}

/// Pure debounce/gesture state machine for ONE button. Fed `(down, now_ms)` each poll;
/// returns at most one event per transition. No allocation, deterministic in tests.
pub struct GestureState {
    down: bool,
    press_ms: u32,
    hold_fired: bool,
    last_click_ms: Option<u32>,
}

impl GestureState {
    pub const fn new() -> Self {
        Self { down: false, press_ms: 0, hold_fired: false, last_click_ms: None }
    }

    /// Advance the machine. `down` = is the button physically pressed now; `now_ms` = a
    /// monotonic millisecond clock. Returns an event on the transition that produced it.
    pub fn update(&mut self, down: bool, now_ms: u32) -> Option<ButtonEvent> {
        if down && !self.down {
            self.down = true;
            self.press_ms = now_ms;
            self.hold_fired = false;
            return None;
        }
        if down && self.down {
            if !self.hold_fired && now_ms.wrapping_sub(self.press_ms) >= HOLD_MS {
                self.hold_fired = true;
                return Some(ButtonEvent::Hold);
            }
            return None;
        }
        if !down && self.down {
            self.down = false;
            if self.hold_fired {
                self.last_click_ms = None;
                return None;
            }
            if let Some(prev) = self.last_click_ms {
                if now_ms.wrapping_sub(prev) <= DOUBLE_MS {
                    self.last_click_ms = None;
                    return Some(ButtonEvent::DoubleClick);
                }
            }
            self.last_click_ms = Some(now_ms);
            return Some(ButtonEvent::Click);
        }
        None
    }
}

impl Default for GestureState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_is_emitted_on_short_press_release() {
        let mut g = GestureState::new();
        assert_eq!(g.update(true, 0), None);
        assert_eq!(g.update(false, 120), Some(ButtonEvent::Click));
    }

    #[test]
    fn hold_is_emitted_once_after_threshold_while_still_down() {
        let mut g = GestureState::new();
        assert_eq!(g.update(true, 0), None);
        assert_eq!(g.update(true, HOLD_MS), Some(ButtonEvent::Hold));
        assert_eq!(g.update(true, HOLD_MS + 500), None);
        assert_eq!(g.update(false, HOLD_MS + 600), None);
    }

    #[test]
    fn double_click_when_second_press_within_window() {
        let mut g = GestureState::new();
        assert_eq!(g.update(true, 0), None);
        assert_eq!(g.update(false, 80), Some(ButtonEvent::Click));
        assert_eq!(g.update(true, 80 + 100), None);
        assert_eq!(g.update(false, 80 + 160), Some(ButtonEvent::DoubleClick));
    }
}
