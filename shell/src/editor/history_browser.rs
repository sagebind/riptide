use crate::editor::{
    event::Event,
    scene::{RenderMode, Scene, SceneContext},
};

#[derive(Default)]
pub struct HistoryBrowser {}

impl Scene for HistoryBrowser {
    fn render_mode(&self) -> RenderMode {
        RenderMode::Alt
    }

    fn on_input(&mut self, ctx: &mut SceneContext, event: Event) {
        match event {
            Event::Char('\n') => {
                ctx.close();
            }

            _ => {}
        }
    }

    fn render(&self) -> String {
        "History...".into()
    }
}
