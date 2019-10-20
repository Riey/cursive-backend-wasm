use cursive::event::Event;
use cursive::{backend, Vec2};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

use crate::shared::Shared;

use cursive::theme::{BaseColor, Color, ColorPair, Effect};
use glyph_brush::{
    rusttype::{PositionedGlyph, Rect, Scale},
    BrushAction, BrushError, Color as GlColor, FontId, GlyphBrush, GlyphBrushBuilder, GlyphVertex,
    Layout, Section,
};
use wasm_bindgen::JsCast;
use web_sys::{OffscreenCanvas, WebGlRenderingContext, WebGlRenderingContext as Gl, WebGlTexture};
type Vertex = [f32; 13];

pub struct Backend {
    shared: Arc<Shared>,
    cursive_color: Cell<ColorPair>,
    color: Cell<GlColor>,
    bg_color: Cell<GlColor>,
    console: OffscreenCanvas,
    ctx: WebGlRenderingContext,
    font_width: f32,
    font_height: f32,
    brush: RefCell<GlyphBrush<'static, Vertex>>,
    texture: Option<WebGlTexture>,
    vertex_max: usize,
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
        let console_size = (console.width(), console.height());
        let brush = GlyphBrushBuilder::using_font_bytes(font)
            .initial_cache_size(console_size)
            .gpu_cache_align_4x4(true)
            .build::<Vertex>();

        Box::new(Self {
            shared,
            cursive_color: Cell::new(ColorPair {
                front: Color::Rgb(255, 255, 255),
                back: Color::Rgb(0, 0, 0),
            }),
            color: Cell::new([1.0, 1.0, 1.0, 1.0]),
            bg_color: Cell::new([0.0, 0.0, 0.0, 1.0]),
            console,
            ctx,
            font_height,
            font_width,
            brush: RefCell::new(brush),
            texture: None,
            vertex_max: 0,
        }) as Box<_>
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        let e = self
            .shared
            .event_buffer
            .lock()
            .unwrap()
            .pop_front()
            .map(|e| e.into_event(self.font_width as _, self.font_height as _));

        log::trace!("Get event {:?}", e);

        e
    }

    fn finish(&mut self) {}

    fn refresh(&mut self) {
        let action = loop {
            match self
                .brush
                .get_mut()
                .process_queued(update_texture, to_vertex)
            {
                Ok(action) => break action,
                Err(BrushError::TextureTooSmall { suggested }) => {
                    self.texture = self.ctx.create_texture();
                    self.ctx.bind_texture(Gl::TEXTURE_2D, self.texture.as_ref());
                    self.brush
                        .get_mut()
                        .resize_texture(suggested.0, suggested.1);
                }
            }
        };

        match action {
            BrushAction::Draw(vertices) => {
                let vertex_count = vertices.len();
                unsafe {
                    if self.vertex_max < vertex_count {
                        self.ctx.buffer_data_with_u8_array(
                            WebGlRenderingContext::ARRAY_BUFFER,
                            vertices.as_ptr() as _,
                            Gl::DYNAMIC_DRAW,
                        );
                    } else {
                        self.ctx.buffer_sub_data_with_i32_and_u8_array(
                            Gl::ARRAY_BUFFER,
                            0,
                            vertices.as_ptr() as _,
                        );
                    }
                }
                self.vertex_max = self.vertex_max.max(vertex_count);
                self.ctx.clear(Gl::COLOR_BUFFER_BIT);
                self.ctx
                    .draw_arrays_instanced(Gl::TRIANGLE_STRIP, 0, 4, vertex_count as i32);

                self.ctx.commit();
            }
            BrushAction::ReDraw => {}
        }
    }

    fn has_colors(&self) -> bool {
        true
    }

    fn screen_size(&self) -> Vec2 {
        Vec2::new(self.console.width() as _, self.console.height() as _)
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        let mut brush = self.brush.borrow_mut();

        brush.queue(Section {
            color: self.color.get(),
            text,
            font_id: FontId(0),
            scale: Scale::uniform(self.font_height),
            bounds: (self.console.width() as _, self.console.height() as _),
            layout: Layout::default_single_line(),
            screen_position: (
                pos.x as f32 * self.font_width,
                pos.y as f32 * self.font_height,
            ),
            ..Section::default()
        });
    }

    fn clear(&self, color: Color) {
        let [r, g, b, a] = to_gl_color(color);
        self.ctx.clear_color(r, g, b, a);
        self.ctx.clear(Gl::COLOR_BUFFER_BIT);
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

fn base_to_dark_gl_color(color: BaseColor) -> GlColor {
    match color {
        BaseColor::Black => [0.0, 0.0, 0.0, 1.0],
        BaseColor::Blue => [0.0, 0.0, 1.0, 1.0],
        BaseColor::Cyan => [0.0, 1.0, 1.0, 1.0],
        BaseColor::Yellow => [1.0, 1.0, 0.0, 1.0],
        BaseColor::Green => [0.0, 1.0, 0.0, 1.0],
        BaseColor::Magenta => [1.0, 0.0, 1.0, 1.0],
        BaseColor::White => [1.0, 1.0, 1.0, 1.0],
        BaseColor::Red => [1.0, 0.0, 0.0, 1.0],
    }
}

fn base_to_light_gl_color(color: BaseColor) -> GlColor {
    match color {
        BaseColor::Black => [0.0, 0.0, 0.0, 1.0],
        BaseColor::Blue => [0.0, 0.0, 1.0, 1.0],
        BaseColor::Cyan => [0.0, 1.0, 1.0, 1.0],
        BaseColor::Yellow => [1.0, 1.0, 0.0, 1.0],
        BaseColor::Green => [0.0, 1.0, 0.0, 1.0],
        BaseColor::Magenta => [1.0, 0.0, 1.0, 1.0],
        BaseColor::White => [1.0, 1.0, 1.0, 1.0],
        BaseColor::Red => [1.0, 0.0, 0.0, 1.0],
    }
}

fn to_gl_color(color: Color) -> GlColor {
    match color {
        Color::Dark(color) => base_to_dark_gl_color(color),
        Color::Light(color) => base_to_light_gl_color(color),
        Color::RgbLowRes(r, g, b) => [r as f32 / 6.0, g as f32 / 6.0, b as f32 / 6.0, 1.0],
        Color::Rgb(r, g, b) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0],
        Color::TerminalDefault => [0.0, 0.0, 0.0, 1.0],
    }
}

fn update_texture(rect: Rect<u32>, tex_data: &[u8]) {}

fn to_vertex(
    GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        color,
        z,
    }: GlyphVertex,
) -> Vertex {
}
