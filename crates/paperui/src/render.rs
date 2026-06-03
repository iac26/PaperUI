use crate::canvas::Canvas;
use crate::geometry::Rect;
use crate::types::UpdateHint;

/// Panel presentation + power lifecycle (spec §5.3). A stateful backend object.
/// `before_draw`/`after_draw` let e-ink wake/sleep its controller; TFT leaves them empty.
pub trait Renderer<C: Canvas> {
    fn begin(&mut self) {}
    fn before_draw(&mut self) {}
    fn present(&mut self, canvas: &mut C, region: Rect, hint: UpdateHint);
    fn after_draw(&mut self) {}
    fn shutdown(&mut self) {}
}
