//! Window/display configuration for the desktop preview.

use paperui::Size;

/// How the preview window is sized. `size` is the emulated panel resolution in PaperUI
/// pixels; `scale` is an integer upscale so the window is visible on a hi-dpi laptop.
pub struct SimConfig {
    pub size: Size,
    pub scale: u32,
    pub title: &'static str,
}

impl SimConfig {
    /// M5StickC Plus2: 135×240 color TFT driven in landscape (rotated 90°), so the logical
    /// surface the UI draws into is 240×135. Upscaled 3×.
    pub fn stickc() -> Self {
        Self { size: Size::new(240, 135), scale: 3, title: "PaperUI — StickC preview" }
    }

    /// M5Paper: 540×960 e-ink panel, 1× (renders the mono theme as grayscale).
    pub fn m5paper() -> Self {
        Self { size: Size::new(540, 960), scale: 1, title: "PaperUI — M5Paper preview" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stickc_preset_matches_the_panel() {
        let c = SimConfig::stickc();
        assert_eq!(c.size, Size::new(240, 135), "landscape (rotated) logical surface");
        assert_eq!(c.scale, 3);
    }
}
