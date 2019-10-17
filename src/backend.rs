use cursive::event::Event;
use cursive::{backend, Vec2};
use std::cell::Cell;
use std::sync::Arc;

use crate::event_handler::WasmEvent;
use crate::shared::Shared;

use cursive::theme::{BaseColor, Color, ColorPair, Effect};
use wasm_bindgen::JsCast;
use web_sys::{OffscreenCanvas, WebGlRenderingContext};

pub struct Backend {
    shared: Arc<Shared>,
    cursive_color: Cell<ColorPair>,
    color: Cell<[f32; 3]>,
    bg_color: Cell<[f32; 3]>,
    console: OffscreenCanvas,
    ctx: WebGlRenderingContext,
    font_width: f32,
    font_height: f32,
    font: Vec<u8>,
}

impl Backend {
    pub fn init(
        console: OffscreenCanvas,
        shared: Arc<Shared>,
        font_height: f32,
        font: Vec<u8>,
    ) -> Box<dyn backend::Backend> {
        let ctx: WebGlRenderingContext = console
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();
        // TODO: measuring text width
        let font_width = font_height * 0.8;

        Box::new(Self {
            shared,
            cursive_color: Cell::new(ColorPair {
                front: Color::Rgb(255, 255, 255),
                back: Color::Rgb(0, 0, 0),
            }),
            color: Cell::new([1.0, 1.0, 1.0]),
            bg_color: Cell::new([0.0, 0.0, 0.0]),
            console,
            ctx,
            font_height,
            font_width,
            font,
        }) as Box<_>
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        self.shared
            .try_lock()
            .unwrap()
            .dequeue()
            .map(|e| e.into_event(self.font_width as _, self.font_height as _))
    }

    fn finish(&mut self) {}

    fn refresh(&mut self) {
        self.ctx.commit();
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> Vec2 {
        Vec2::new(self.console.width() as _, self.console.height() as _)
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        // TODO: implement
    }

    fn clear(&self, color: Color) {
        let [r, g, b] = to_gl_color(color);
        self.ctx.clear_color(r, g, b, 1.0);
        self.ctx.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    }

    fn set_color(&self, colors: ColorPair) -> ColorPair {
        self.color.set(to_gl_color(colors.front));
        self.bg_color.set(to_gl_color(colors.back));
        self.cursive_color.replace(colors)
    }

    fn set_effect(&self, effect: Effect) {
        // TODO: implement
    }

    fn unset_effect(&self, effect: Effect) {
        // TODO: implement
    }
}

fn to_gl_color(color: Color) -> [f32; 3] {
    match color {
        Color::Dark(BaseColor::Black) => [0.0, 0.0, 0.0],
        Color::Dark(BaseColor::Blue) => [0.0, 0.0, 0.9],
        Color::Dark(BaseColor::Cyan) => [0.0, 0.9, 0.9],
        Color::Dark(BaseColor::Yellow) => [0.9, 0.9, 0.0],
        Color::RgbLowRes(r, g, b) => [r as f32 / 6.0, g as f32 / 6.0, b as f32 / 6.0],
        Color::Rgb(r, g, b) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
        _ => unimplemented!(),
    }
}
