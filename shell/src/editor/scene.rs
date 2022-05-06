use crate::editor::event::Event;

pub trait Scene {
    fn render_mode(&self) -> RenderMode;

    fn on_input(&mut self, ctx: &mut SceneContext, event: Event);

    fn render(&self) -> String;
}

#[derive(Default)]
pub struct SceneContext {
    pub(crate) close: bool,
}

impl SceneContext {
    pub fn close(&mut self) {
        self.close = true;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderMode {
    Normal,
    Alt,
}
