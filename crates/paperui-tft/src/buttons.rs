//! Reads the StickC's BtnA/BtnB GPIOs (active-low) and turns them into gesture events.
use esp_hal::gpio::Input;
use paperui_core::{ButtonEvent, ButtonId, GestureState};

pub struct ButtonReader<'d> {
    btn_a: Input<'d>,
    btn_b: Input<'d>,
    g_a: GestureState,
    g_b: GestureState,
}

impl<'d> ButtonReader<'d> {
    pub fn new(btn_a: Input<'d>, btn_b: Input<'d>) -> Self {
        Self {
            btn_a,
            btn_b,
            g_a: GestureState::new(),
            g_b: GestureState::new(),
        }
    }

    /// Poll both at now_ms; returns first event (A before B). Pressed == pin low.
    pub fn poll(&mut self, now_ms: u32) -> Option<(ButtonId, ButtonEvent)> {
        let a_down = self.btn_a.is_low();
        let b_down = self.btn_b.is_low();
        if let Some(ev) = self.g_a.update(a_down, now_ms) {
            return Some((ButtonId::A, ev));
        }
        if let Some(ev) = self.g_b.update(b_down, now_ms) {
            return Some((ButtonId::B, ev));
        }
        None
    }
}
