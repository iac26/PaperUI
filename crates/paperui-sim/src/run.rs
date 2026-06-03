//! The windowed run loop: build a SimulatorDisplay, paint the first full frame, then pump
//! window events into the reactive core and surgically repaint when something is dirty.

use std::{thread, time::Duration};

use embedded_graphics::geometry::Size as EgSize;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics_simulator::{
    sdl2::Keycode, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};

use paperui::reactive::{
    dispatch, has_dirty, layout, render_frame, render_frame_full, NodeId, UiEvent,
};
use paperui::{EgCanvas, Rect, WidgetTheme};

use crate::config::SimConfig;
use crate::input_map::{map_key, to_point};

/// Open a window and run the reactive tree `root` (already built under a `Scope`) until the
/// window closes. Generic over the theme exactly like the embedded `run()`, so it hosts
/// `TftTheme`, `EinkTheme`, or any `WidgetTheme`.
pub fn run_sim<T>(root: NodeId, theme: &T, cfg: SimConfig)
where
    T: for<'a> WidgetTheme<EgCanvas<'a, SimulatorDisplay<Rgb565>>>,
{
    let mut display =
        SimulatorDisplay::<Rgb565>::new(EgSize::new(cfg.size.w as u32, cfg.size.h as u32));

    // Lay the tree out to fill the panel, then paint the first full frame.
    layout(root, Rect::new(0, 0, cfg.size.w, cfg.size.h));
    {
        let mut canvas = EgCanvas::new(&mut display);
        render_frame_full(root, &mut canvas, theme);
    }

    let output_settings = OutputSettingsBuilder::new().scale(cfg.scale).build();
    let mut window = Window::new(cfg.title, &output_settings);

    'running: loop {
        window.update(&display);
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::KeyDown { keycode, .. } => {
                    if keycode == Keycode::Escape {
                        break 'running;
                    }
                    if let Some(ev) = map_key(keycode) {
                        dispatch(ev);
                    }
                }
                SimulatorEvent::MouseButtonDown { point, .. } => {
                    dispatch(UiEvent::Pointer(to_point(point)));
                }
                _ => {}
            }
        }
        if has_dirty() {
            let mut canvas = EgCanvas::new(&mut display);
            render_frame(root, &mut canvas, theme);
        }
        thread::sleep(Duration::from_millis(16)); // ~60 Hz; avoid a busy spin
    }
}
