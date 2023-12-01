use glow::Context;

pub trait Effect {
    fn render(&self, gl: &Context, time: f32);
}