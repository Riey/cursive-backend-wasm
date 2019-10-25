use cursive::backend;
use cursive::event::Event;
use cursive::theme::{BaseColor, Color, ColorPair, Effect};
use cursive::Vec2;
use std::cell::{Cell, RefCell};
use unicode_width::UnicodeWidthStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, ContextAttributes2d, DedicatedWorkerGlobalScope, OffscreenCanvas,
};

use crate::event_handler::WasmEvent;
use crate::shared::Shared;

struct ColorCache {
    color: JsValue,
    bg_color: JsValue,
}

impl Default for ColorCache {
    fn default() -> Self {
        Self {
            color: JsValue::UNDEFINED,
            bg_color: JsValue::UNDEFINED,
        }
    }
}

pub struct Backend {
    shared: Shared,
    event_buffer: Vec<WasmEvent>,
    color: Cell<ColorPair>,
    color_cache: RefCell<ColorCache>,
    cur_bg_color: RefCell<JsValue>,
    effect: Cell<Effect>,

    font_width: f64,
    font_height: f64,
    console: OffscreenCanvas,
    ctx: CanvasRenderingContext2d,
    global: DedicatedWorkerGlobalScope,
}

impl Backend {
    pub fn init(
        console: OffscreenCanvas,
        font_family: &str,
        font_size: f64,
        shared: Shared,
    ) -> Result<Box<dyn backend::Backend>, JsValue> {
        let ctx: CanvasRenderingContext2d = console
            .get_context_with_context_options(
                "2d",
                ContextAttributes2d::new().alpha(false).as_ref(),
            )?
            .ok_or("Can't get CanvasRenderingContext2d")?
            .unchecked_into();

        ctx.set_font(&format!("{}px {}", font_size, font_family));
        ctx.set_text_baseline("top");

        let font_width = ctx.measure_text("M")?.width();

        Ok(Box::new(Self {
            shared,
            event_buffer: Vec::with_capacity(100),
            console,
            ctx,
            font_width,
            font_height: font_size,
            color: Cell::new(ColorPair {
                front: Color::TerminalDefault,
                back: Color::TerminalDefault,
            }),
            cur_bg_color: RefCell::new(JsValue::UNDEFINED),
            color_cache: RefCell::default(),
            effect: Cell::new(Effect::Simple),
            global: js_sys::global().dyn_into()?,
        }))
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        if self.event_buffer.is_empty() {
            self.shared.pop(&mut self.event_buffer);
        }
        self.event_buffer.pop().map(|e| {
            if let WasmEvent::Resize(x, y) = e {
                self.console.set_width(x);
                self.console.set_height(y);
            }
            e.into_event(self.font_width as usize, self.font_height as usize)
        })
    }

    fn clear(&self, color: Color) {
        log::trace!("clear color: {:?}", color);
        let color = color_to_html(color).into();
        self.ctx.set_fill_style(&color);
        self.ctx.fill_rect(
            0.,
            0.,
            self.console.width() as _,
            self.console.height() as _,
        );
        *self.cur_bg_color.borrow_mut() = color;
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        let color_cache = self.color_cache.borrow();

        let x = pos.x as f64 * self.font_width;
        let y = pos.y as f64 * self.font_height;
        let width = self.font_width * text.width() as f64;

        self.ctx.set_fill_style(&color_cache.bg_color);
        self.ctx.fill_rect(x, y, width, self.font_height);
        self.ctx.set_fill_style(&color_cache.color);
        self.ctx.fill_text(text, x, y).unwrap();
    }

    fn refresh(&mut self) {
        let image = self.console.transfer_to_image_bitmap().unwrap();
        self.global
            .post_message_with_transfer(image.as_ref(), image.as_ref())
            .unwrap();

        self.ctx.set_fill_style(&self.cur_bg_color.borrow());
        self.ctx.fill_rect(
            0.,
            0.,
            self.console.width() as _,
            self.console.height() as _,
        );
    }

    fn screen_size(&self) -> Vec2 {
        Vec2::new(
            self.console.width() as usize / self.font_width as usize,
            self.console.height() as usize / self.font_height as usize,
        )
    }

    fn finish(&mut self) {}

    fn has_colors(&self) -> bool {
        true
    }

    fn set_color(&self, colors: ColorPair) -> ColorPair {
        let old = self.color.replace(colors);

        let mut color_cache = self.color_cache.borrow_mut();

        color_cache.color = color_to_html(colors.front).into();
        color_cache.bg_color = color_to_html(colors.back).into();

        old
    }

    fn set_effect(&self, effect: Effect) {
        self.effect.set(effect);
    }

    fn unset_effect(&self, _effect: Effect) {
        self.effect.set(Effect::Simple);
    }
}

fn light_base_color_to_html(color: BaseColor) -> &'static str {
    match color {
        BaseColor::Black => "Gray",
        BaseColor::Blue => "LightBlue",
        BaseColor::Cyan => "LightCyan",
        BaseColor::Green => "LightGreen",
        BaseColor::Magenta => "Magenta",
        BaseColor::Red => "LightRed",
        BaseColor::White => "White",
        BaseColor::Yellow => "LightYellow",
    }
}

fn dark_base_color_to_html(color: BaseColor) -> &'static str {
    match color {
        BaseColor::Black => "Black",
        BaseColor::Blue => "Blue",
        BaseColor::Cyan => "Cyan",
        BaseColor::Green => "Green",
        BaseColor::Magenta => "LightMagenta",
        BaseColor::Red => "Red",
        BaseColor::White => "White",
        BaseColor::Yellow => "Yellow",
    }
}

fn rgb_to_html(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn low_res_to_high(c: u8) -> u8 {
    (c as u32 * 255 / 6) as u8
}

fn color_to_html(color: Color) -> String {
    match color {
        Color::TerminalDefault => "inherit".into(),
        Color::Dark(color) => dark_base_color_to_html(color).into(),
        Color::Light(color) => light_base_color_to_html(color).into(),
        Color::Rgb(r, g, b) => rgb_to_html(r, g, b),
        Color::RgbLowRes(r, g, b) => {
            rgb_to_html(low_res_to_high(r), low_res_to_high(g), low_res_to_high(b))
        }
    }
}
