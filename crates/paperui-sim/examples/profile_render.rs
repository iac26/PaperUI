//! Headless profiling harness for the PaperUI render paths (see the static analysis).
//! Builds a representative tree (signal-driven counter text + a 6-item carousel), then
//! times each rendering path on a real `SimulatorDisplay<Rgb565>` and, separately, tallies
//! pixels/primitive-calls via a counting `DrawTarget` (the hardware-independent SPI-cost
//! proxy: device frame time tracks pixels pushed). No window is opened. Run with:
//!   cargo run -p paperui-sim --example profile_render --target x86_64-unknown-linux-gnu --release

use std::time::Instant;

use embedded_graphics::geometry::Size as EgSize;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_simulator::SimulatorDisplay;

use paperui::reactive::{
    button, carousel, carousel_select_first, col, dispatch, install, layout, render_frame_full,
    render_tick, tick_anims, text, Scope, Storage, UiEvent,
};
use paperui::{EgCanvas, Rect};
use paperui_tft::TftTheme;
use static_cell::StaticCell;

const W: i16 = 240;
const H: i16 = 135;
const PANEL_PX: u64 = (W as u64) * (H as u64);

// ---- counting DrawTarget: tallies pixels + primitive calls, draws nothing ----
struct Counter {
    pixels: u64,
    fills: u64,
    contigs: u64,
    iters: u64,
}
impl Counter {
    const fn new() -> Self {
        Self { pixels: 0, fills: 0, contigs: 0, iters: 0 }
    }
    fn reset(&mut self) {
        *self = Counter::new();
    }
}
impl OriginDimensions for Counter {
    fn size(&self) -> EgSize {
        EgSize::new(W as u32, H as u32)
    }
}
impl DrawTarget for Counter {
    type Color = Rgb565;
    type Error = core::convert::Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, px: I) -> Result<(), Self::Error> {
        self.pixels += px.into_iter().count() as u64;
        self.iters += 1;
        Ok(())
    }
    // Override the fill fast-paths so we tally area arithmetically (and so they don't funnel
    // through draw_iter): this mirrors what an efficient real DrawTarget does.
    fn fill_solid(&mut self, area: &Rectangle, _c: Rgb565) -> Result<(), Self::Error> {
        self.pixels += (area.size.width * area.size.height) as u64;
        self.fills += 1;
        Ok(())
    }
    fn fill_contiguous<I: IntoIterator<Item = Rgb565>>(
        &mut self,
        area: &Rectangle,
        _c: I,
    ) -> Result<(), Self::Error> {
        self.pixels += (area.size.width * area.size.height) as u64;
        self.contigs += 1;
        Ok(())
    }
}

fn time_it(iters: u32, mut f: impl FnMut()) -> f64 {
    for _ in 0..(iters / 8).max(1) {
        f();
    }
    let t = Instant::now();
    for _ in 0..iters {
        f();
    }
    t.elapsed().as_nanos() as f64 / iters as f64
}

fn main() {
    // 1 signal, 9 nodes (counter text + 6 buttons + carousel + col), 7 effects
    // (counter effect + 6 button handlers), 1 owner, 1 anim (slide tween).
    static STORAGE: StaticCell<Storage<1, 9, 7, 1, 1>> = StaticCell::new();
    install(STORAGE.init(Storage::new()));
    let cx = Scope::root();

    let count = cx.signal(0i32);
    let counter = text(cx, move || {
        let mut s = heapless::String::<32>::new();
        let _ = core::fmt::write(&mut s, format_args!("Count: {}", count.get()));
        s
    });
    let items = [
        button(cx, "Living Room", || {}),
        button(cx, "Kitchen", || {}),
        button(cx, "Bedroom", || {}),
        button(cx, "Office", || {}),
        button(cx, "Garage", || {}),
        button(cx, "Garden", || {}),
    ];
    let rooms = carousel(cx, &items);
    let root = col(cx, (counter, rooms));
    carousel_select_first(rooms);

    let theme = TftTheme;
    let area = Rect::new(0, 0, W, H);
    let mut sim = SimulatorDisplay::<Rgb565>::new(EgSize::new(W as u32, H as u32));
    let mut counter_tgt = Counter::new();

    // settle into a known, clean state: full paint clears the dirty set.
    layout(root, area);
    {
        let mut c = EgCanvas::new(&mut sim);
        render_frame_full(root, &mut c, &theme);
    }

    println!("panel = {W}x{H} = {PANEL_PX} px\n");

    // ---- pixel & primitive tally (deterministic; one representative call each) ----
    println!("== pixels & primitive-calls per operation ==");
    println!(
        "{:<14} {:>9} {:>6} {:>7} {:>6} {:>8}",
        "path", "pixels", "fills", "contig", "iters", "x panel"
    );

    let tally = |name: &str, ct: &Counter| {
        println!(
            "{:<14} {:>9} {:>6} {:>7} {:>6} {:>7.2}x",
            name,
            ct.pixels,
            ct.fills,
            ct.contigs,
            ct.iters,
            ct.pixels as f64 / PANEL_PX as f64
        );
    };

    // full_frame
    counter_tgt.reset();
    {
        let mut c = EgCanvas::new(&mut counter_tgt);
        render_frame_full(root, &mut c, &theme);
    }
    tally("full_frame", &counter_tgt);

    // surgical text update (signal change -> 1 dirty text node)
    count.update(|v| *v += 1);
    counter_tgt.reset();
    {
        let mut c = EgCanvas::new(&mut counter_tgt);
        render_tick(root, &mut c, &theme); // not animating + dirty => render_frame (surgical)
    }
    tally("surgical_text", &counter_tgt);

    // idle tick (nothing dirty, nothing animating)
    counter_tgt.reset();
    {
        let mut c = EgCanvas::new(&mut counter_tgt);
        render_tick(root, &mut c, &theme);
    }
    tally("idle_tick", &counter_tgt);

    // animated carousel scroll: arm one slide, sweep its 300ms duration in ~16ms steps,
    // tally total pixels written across the full animation.
    // The slide duration is 300 ms (as set in the carousel builder).
    const SLIDE_MS: u32 = 300;
    const STEP_MS: u32 = 16;
    dispatch(UiEvent::FocusNext); // arms the slide tween
    counter_tgt.reset();
    for now in (0..=SLIDE_MS).step_by(STEP_MS as usize) {
        tick_anims(now);
        let mut c = EgCanvas::new(&mut counter_tgt);
        render_tick(root, &mut c, &theme);
    }
    tally("anim_scroll", &counter_tgt);
    // drain any remainder so we return to settled state before timing
    tick_anims(SLIDE_MS + 1);
    {
        let mut c = EgCanvas::new(&mut sim);
        render_tick(root, &mut c, &theme);
    }

    // ---- wall-clock timing on the real Rgb565 buffer (release build only is meaningful) ----
    println!("\n== wall-clock per operation (SimulatorDisplay<Rgb565>) ==");
    println!("{:<16} {:>12}", "path", "ns/op");

    let ns_layout = time_it(50_000, || layout(root, area));
    println!("{:<16} {:>12.1}", "layout", ns_layout);

    let ns_full = time_it(20_000, || {
        let mut c = EgCanvas::new(&mut sim);
        render_frame_full(root, &mut c, &theme);
    });
    println!("{:<16} {:>12.1}", "full_frame", ns_full);

    // surgical: dirty the text each iteration, then tick (includes the full relayout).
    let ns_surgical = time_it(20_000, || {
        count.update(|v| *v += 1);
        let mut c = EgCanvas::new(&mut sim);
        render_tick(root, &mut c, &theme);
    });
    println!("{:<16} {:>12.1}", "surgical_text", ns_surgical);

    // idle: ensure clean first, then time the no-op tick path.
    {
        let mut c = EgCanvas::new(&mut sim);
        render_frame_full(root, &mut c, &theme);
    }
    let ns_idle = time_it(50_000, || {
        let mut c = EgCanvas::new(&mut sim);
        render_tick(root, &mut c, &theme);
    });
    println!("{:<16} {:>12.1}", "idle_tick", ns_idle);

    // one full carousel scroll = arm + sweep 300ms in 16ms steps (~19 frames).
    let n_frames = (SLIDE_MS / STEP_MS + 1) as usize;
    let mut anim_t = 0u32;
    let ns_scroll = time_it(10_000, || {
        dispatch(UiEvent::FocusNext);
        anim_t = 0;
        loop {
            tick_anims(anim_t);
            let mut c = EgCanvas::new(&mut sim);
            render_tick(root, &mut c, &theme);
            if anim_t >= SLIDE_MS {
                break;
            }
            anim_t = (anim_t + STEP_MS).min(SLIDE_MS);
        }
    });
    println!(
        "{:<16} {:>12.1}   (~{} frames; {:.1} ns/frame)",
        "anim_scroll",
        ns_scroll,
        n_frames,
        ns_scroll / n_frames as f64
    );
}
