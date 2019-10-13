use cursive::backend;
use cursive::event::{Event, Key, MouseButton, MouseEvent as CursiveMouseEvent};
use cursive::theme::{BaseColor, Color, ColorPair, Effect};
use cursive::Vec2;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use unicode_width::UnicodeWidthStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    window, CanvasRenderingContext2d, CompositionEvent, ContextAttributes2d, EventTarget,
    HtmlCanvasElement, HtmlElement, HtmlInputElement, KeyboardEvent, MouseEvent, TouchEvent,
};

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
    event_buffer: Rc<RefCell<VecDeque<Event>>>,
    color: Cell<ColorPair>,
    color_cache: RefCell<ColorCache>,
    cur_bg_color: RefCell<JsValue>,
    effect: Cell<Effect>,

    font_width: f64,
    font_height: f64,
    console: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    _input: HtmlInputElement,

    _closures: Vec<Closure<dyn Fn()>>,
    _mouse_closures: Vec<Closure<dyn Fn(MouseEvent)>>,
    _touch_closures: Vec<Closure<dyn Fn(TouchEvent)>>,
    _keyboard_closures: Vec<Closure<dyn Fn(KeyboardEvent)>>,
    _composition_closures: Vec<Closure<dyn Fn(CompositionEvent)>>,
}

impl Backend {
    pub fn init(
        console: HtmlCanvasElement,
        composition_text: HtmlElement,
        font_family: &str,
        font_size: f64,
    ) -> Result<Box<dyn backend::Backend>, JsValue> {
        let window = window().ok_or("Window isn't exist")?;
        let document = window.document().ok_or("Document isn't exist")?;

        let input: HtmlInputElement = document.create_element("input")?.unchecked_into();
        console.append_child(&input)?;
        input.set_autofocus(true);
        input.style().set_property("position", "absolute")?;
        input.style().set_property("top", "0px")?;
        input.style().set_property("left", "0px")?;
        input.style().set_property("border", "none")?;
        input.style().set_property("width", "100%")?;
        input.style().set_property("height", "100%")?;
        input.style().set_property("opacity", "0")?;
        input.style().set_property("z-index", "0")?;
        input.style().set_property("padding", "0px")?;
        input.style().set_property("pointer-events", "none")?;
        input.focus()?;

        let ctx: CanvasRenderingContext2d = console
            .get_context_with_context_options(
                "2d",
                ContextAttributes2d::new().alpha(false).as_ref(),
            )?
            .ok_or("Can't get CanvasRenderingContext2d")?
            .dyn_into()?;
        ctx.set_font(&format!("{}px {}", font_size, font_family));
        ctx.set_text_baseline("top");

        let height = font_size;
        let width = ctx.measure_text("M")?.width();

        let mut closures = Vec::with_capacity(1);
        let mut mouse_closures = Vec::with_capacity(3);
        let mut touch_closures = Vec::with_capacity(3);
        let mut keyboard_closures = Vec::with_capacity(1);
        let mut composition_closures = Vec::with_capacity(2);
        let event_buffer = Rc::new(RefCell::new(VecDeque::with_capacity(300)));
        let hold_start = Rc::new(Cell::new(false));

        {
            //let console_inner = console.clone();
            //let offscreen_console = offscreen_console.clone();
            let event_buffer = event_buffer.clone();
            let onresize = Closure::wrap(Box::new(move || {
                event_buffer.borrow_mut().push_back(Event::WindowResize);
                //offscreen_console.set_width(console_inner.width());
                //offscreen_console.set_height(console_inner.height());
            }) as Box<dyn Fn()>);
            console.set_onresize(Some(onresize.as_ref().unchecked_ref()));

            closures.push(onresize);
        }

        {
            let input = input.clone();
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmousedown = Closure::wrap(Box::new(move |e: MouseEvent| {
                prevent_default(&e);
                input.focus().unwrap();
                hold_start.set(true);
                event_buffer.borrow_mut().push_back(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(e.x() as usize, e.y() as usize),
                    event: CursiveMouseEvent::Press(get_mouse_botton(&e)),
                });
                let e: &web_sys::Event = e.as_ref();
                e.prevent_default();
            }) as Box<dyn Fn(MouseEvent)>);
            console.set_onmousedown(Some(onmousedown.as_ref().unchecked_ref()));

            mouse_closures.push(onmousedown);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmousehold = Closure::wrap(Box::new(move |e: MouseEvent| {
                if !hold_start.get() {
                    return;
                }
                prevent_default(&e);
                event_buffer.borrow_mut().push_back(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(e.x() as usize, e.y() as usize),
                    event: CursiveMouseEvent::Hold(get_mouse_botton(&e)),
                });
            }) as Box<dyn Fn(MouseEvent)>);
            console.set_onmousemove(Some(onmousehold.as_ref().unchecked_ref()));

            mouse_closures.push(onmousehold);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmouseup = Closure::wrap(Box::new(move |e: MouseEvent| {
                prevent_default(&e);
                hold_start.set(false);
                event_buffer.borrow_mut().push_back(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(e.x() as usize, e.y() as usize),
                    event: CursiveMouseEvent::Release(get_mouse_botton(&e)),
                });
            }) as Box<dyn Fn(MouseEvent)>);
            console.set_onmouseup(Some(onmouseup.as_ref().unchecked_ref()));

            mouse_closures.push(onmouseup);
        }

        {
            let event_buffer = event_buffer.clone();
            let onkeydown = Closure::wrap(Box::new(move |e: KeyboardEvent| {
                prevent_default(&e);
                let key_str = e.key();
                log::trace!("keydown: [{}]", key_str);
                let key_str = key_str.as_bytes();
                let key = match key_str {
                    b"Backspace" => Some(Key::Backspace),
                    b"Tab" => Some(Key::Tab),
                    b"Enter" => Some(Key::Enter),
                    b"Esc" => Some(Key::Esc),
                    b"Insert" => Some(Key::Ins),
                    b"Delete" => Some(Key::Del),
                    b"ArrowDown" => Some(Key::Down),
                    b"ArrowUp" => Some(Key::Up),
                    b"ArrowLeft" => Some(Key::Left),
                    b"ArrowRight" => Some(Key::Right),
                    b"Process" => return,
                    _ => None,
                };

                if let Some(key) = key {
                    //TODO: alt ctrl shift meta
                    event_buffer.borrow_mut().push_back(Event::Key(key));
                } else if key_str.len() == 1 {
                    event_buffer
                        .borrow_mut()
                        .push_back(Event::Char(key_str[0] as char));
                };
            }) as Box<dyn Fn(KeyboardEvent)>);
            input.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));

            keyboard_closures.push(onkeydown);
        }

        {
            let target: &EventTarget = input.as_ref();
            let composition_text = composition_text.clone();
            let oncompositionupdate = Closure::wrap(Box::new(move |e: CompositionEvent| {
                prevent_default(&e);
                let data = e.data().unwrap();

                log::trace!("compositionupdate: [{}]", data);

                composition_text.set_inner_text(&data);
            })
                as Box<dyn Fn(CompositionEvent)>);
            target.add_event_listener_with_callback(
                "compositionupdate",
                oncompositionupdate.as_ref().unchecked_ref(),
            )?;

            composition_closures.push(oncompositionupdate);
        }
        {
            let target: &EventTarget = input.as_ref();
            let event_buffer = event_buffer.clone();
            let composition_text = composition_text.clone();
            let oncompositionend = Closure::wrap(Box::new(move |e: CompositionEvent| {
                prevent_default(&e);
                let data = e.data().unwrap();

                log::trace!("compositionend: [{}]", data);

                let mut event_buffer = event_buffer.borrow_mut();
                for ch in data.chars() {
                    event_buffer.push_back(Event::Char(ch));
                }

                composition_text.set_inner_text("");
            }) as Box<dyn Fn(CompositionEvent)>);
            target.add_event_listener_with_callback(
                "compositionend",
                oncompositionend.as_ref().unchecked_ref(),
            )?;

            composition_closures.push(oncompositionend);
        }

        {
            let input = input.clone();
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let ontouchstart = Closure::wrap(Box::new(move |e: TouchEvent| {
                let touches = e.touches();

                log::debug!("touch length: {}", touches.length());

                if touches.length() > 1 {
                    log::debug!("Detect multi touch! will be ignored");
                    return;
                }

                if touches.length() == 0 {
                    return;
                }
                input.focus().unwrap();
                input.select();
                prevent_default(&e);

                let touch = e.touches().get(0).unwrap();
                hold_start.set(true);
                event_buffer.borrow_mut().push_back(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(touch.client_x() as usize, touch.client_y() as usize),
                    event: CursiveMouseEvent::Press(MouseButton::Left),
                });
            }) as Box<dyn Fn(TouchEvent)>);
            console.set_ontouchstart(Some(ontouchstart.as_ref().unchecked_ref()));

            touch_closures.push(ontouchstart);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let ontouchmove = Closure::wrap(Box::new(move |e: TouchEvent| {
                let touches = e.touches();

                if touches.length() > 1 {
                    log::debug!("Detect multi touch! will be ignored");
                    return;
                }

                if touches.length() == 0 {
                    return;
                }

                if !hold_start.get() {
                    return;
                }

                let touch = e.touches().get(0).unwrap();
                prevent_default(&e);

                event_buffer.borrow_mut().push_back(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(touch.client_x() as usize, touch.client_y() as usize),
                    event: CursiveMouseEvent::Hold(MouseButton::Left),
                });
            }) as Box<dyn Fn(TouchEvent)>);
            console.set_ontouchmove(Some(ontouchmove.as_ref().unchecked_ref()));

            touch_closures.push(ontouchmove);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let ontouchend = Closure::wrap(Box::new(move |e: TouchEvent| {
                let touches = e.touches();

                if touches.length() > 1 {
                    log::debug!("Detect multi touch! will be ignored");
                    return;
                }

                if touches.length() == 0 {
                    return;
                }

                let touch = e.touches().get(0).unwrap();
                prevent_default(&e);
                hold_start.set(false);
                event_buffer.borrow_mut().push_back(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(touch.client_x() as usize, touch.client_y() as usize),
                    event: CursiveMouseEvent::Release(MouseButton::Left),
                });
            }) as Box<dyn Fn(TouchEvent)>);
            console.set_ontouchend(Some(ontouchend.as_ref().unchecked_ref()));

            touch_closures.push(ontouchend);
        }

        Ok(Box::new(Self {
            console,
            ctx,
            _input: input,
            font_width: width,
            font_height: height,
            event_buffer,
            color: Cell::new(ColorPair {
                front: Color::TerminalDefault,
                back: Color::TerminalDefault,
            }),
            cur_bg_color: RefCell::new(JsValue::UNDEFINED),
            color_cache: RefCell::default(),
            effect: Cell::new(Effect::Simple),
            _closures: closures,
            _mouse_closures: mouse_closures,
            _touch_closures: touch_closures,
            _keyboard_closures: keyboard_closures,
            _composition_closures: composition_closures,
        }))
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        self.event_buffer.borrow_mut().pop_front()
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

    fn refresh(&mut self) {}

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

fn prevent_default(e: &impl AsRef<web_sys::Event>) {
    e.as_ref().prevent_default();
}

fn get_mouse_botton(e: &MouseEvent) -> MouseButton {
    match e.button() {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::Button4,
        4 => MouseButton::Button5,
        _ => MouseButton::Other,
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
    (c as u32 * 256 / 6) as u8
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
