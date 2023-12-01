use glow::Context;
use crate::effect::Effect;

struct EffectEntry<F> where F: Effect + Sized {
    effect: F,
    start: f32,
    end: f32,
}

pub struct Scheduler<F> where F: Effect + Sized {
    scenes: Vec<EffectEntry<F>>,
}

impl<F> Scheduler<F> where F: Effect + Sized {
    pub fn new() -> Self {
        Self {
            scenes: vec![],
        }
    }

    pub fn add_effect(&mut self, start: f32, end: f32, effect: F) {
        self.scenes.push(EffectEntry {
            effect,
            start,
            end,
        });
    }

    pub fn render(&self, gl: &Context, time: f32) {
        for entry in self.scenes.iter().filter(|entry| entry.start <= time && entry.end >= time) {
            entry.effect.render(gl, time);
        }
    }
}
