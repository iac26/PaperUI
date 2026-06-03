//! Pure mappings from simulator input to PaperUI events, kept separate from the windowed
//! loop so they can be unit-tested without opening a window.

use embedded_graphics_simulator::sdl2::Keycode;
use paperui::reactive::UiEvent;
use paperui::Point;

/// Map a key press to a UI event. Tab/Right/Down navigate focus; Enter/Space activate.
/// `Esc` is intentionally NOT mapped here — the loop matches it directly to quit.
pub(crate) fn map_key(k: Keycode) -> Option<UiEvent> {
    match k {
        Keycode::Tab | Keycode::Right | Keycode::Down => Some(UiEvent::FocusNext),
        Keycode::Return | Keycode::KpEnter | Keycode::Space => Some(UiEvent::Activate),
        _ => None,
    }
}

/// Simulator mouse coordinates are already in display space; narrow i32 → i16.
pub(crate) fn to_point(p: embedded_graphics::geometry::Point) -> Point {
    Point::new(p.x as i16, p.y as i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_to_focus_and_activate() {
        assert_eq!(map_key(Keycode::Tab), Some(UiEvent::FocusNext));
        assert_eq!(map_key(Keycode::Right), Some(UiEvent::FocusNext));
        assert_eq!(map_key(Keycode::Down), Some(UiEvent::FocusNext));
        assert_eq!(map_key(Keycode::Return), Some(UiEvent::Activate));
        assert_eq!(map_key(Keycode::Space), Some(UiEvent::Activate));
        assert_eq!(map_key(Keycode::A), None);
    }

    #[test]
    fn mouse_point_narrows_to_paperui_point() {
        let p = embedded_graphics::geometry::Point::new(13, 27);
        assert_eq!(to_point(p), Point::new(13, 27));
    }
}
