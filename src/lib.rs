use cursive::backend;
use cursive::event::{Event, Key, MouseButton, MouseEvent as CursiveMouseEvent};
use cursive::theme::{BaseColor, Color, ColorPair, Effect};
use cursive::Vec2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    window, Document, EventTarget, HtmlDivElement, HtmlSpanElement, HtmlInputElement,
    CompositionEvent, KeyboardEvent, MouseEvent, TouchEvent,
};

struct ColorCache {
    color: String,
    bg_color: String,
}

impl Default for ColorCache {
    fn default() -> Self {
        Self {
            color: "".into(),
            bg_color: "".into(),
        }
    }
}

pub struct Backend {
    event_buffer: Rc<RefCell<Vec<Event>>>,
    color: Cell<ColorPair>,
    color_cache: RefCell<ColorCache>,
    cur_bg_color: RefCell<String>,
    effect: Cell<Effect>,

    font_width: usize,
    font_height: usize,
    console: HtmlDivElement,
    _input: HtmlInputElement,
    document: Document,

    _closures: Vec<Closure<dyn Fn()>>,
    _mouse_closures: Vec<Closure<dyn Fn(MouseEvent)>>,
    _touch_closures: Vec<Closure<dyn Fn(TouchEvent)>>,
    _keyboard_closures: Vec<Closure<dyn Fn(KeyboardEvent)>>,
    _composition_closures: Vec<Closure<dyn Fn(CompositionEvent)>>,
}

impl Backend {
    pub fn init(
        console: HtmlDivElement,
    ) -> Result<Box<dyn backend::Backend>, JsValue> {
        let window = window().ok_or("Window isn't exist")?;
        let document = window.document().ok_or("Document isn't exist")?;

        let temp: HtmlSpanElement = document.create_element("span")?.unchecked_into();
        temp.set_inner_text("\u{2588}");
        console.append_child(&temp)?;
        let width = temp.offset_width() as usize;
        let height = temp.offset_height() as usize;
        console.remove_child(&temp)?;

        let input: HtmlInputElement = document.create_element("input")?.unchecked_into();
        console.append_child(&input)?;
        input.set_autofocus(true);
        input.style().set_property("position", "relative")?;
        input.style().set_property("top", "0px")?;
        input.style().set_property("left", "0px")?;
        input.style().set_property("border", "none")?;
        input.style().set_property("width", "100%")?;
        input.style().set_property("height", "100%")?;
        input.style().set_property("opacity", "0")?;
        input.style().set_property("padding", "0px")?;
        input.style().set_property("pointerEvents", "none")?;

        let mut closures = Vec::with_capacity(1);
        let mut mouse_closures = Vec::with_capacity(3);
        let touch_closures = Vec::with_capacity(3);
        let mut keyboard_closures = Vec::with_capacity(1);
        let mut composition_closures = Vec::with_capacity(1);
        let event_buffer = Rc::new(RefCell::new(Vec::with_capacity(300)));
        let hold_start = Rc::new(Cell::new(false));

        {
            let event_buffer = event_buffer.clone();
            let onresize = Closure::wrap(Box::new(move || {
                event_buffer.borrow_mut().push(Event::WindowResize);
            }) as Box<dyn Fn()>);
            console.set_onresize(Some(onresize.as_ref().unchecked_ref()));

            closures.push(onresize);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmousedown = Closure::wrap(Box::new(move |e: MouseEvent| {
                hold_start.set(true);
                event_buffer.borrow_mut().push(Event::Mouse {
                    offset: Vec2::new(0, 0),
                    position: Vec2::new(e.x() as usize, e.y() as usize),
                    event: CursiveMouseEvent::Press(get_mouse_botton(&e)),
                });
            }) as Box<dyn Fn(MouseEvent)>);
            console.set_onmousedown(Some(onmousedown.as_ref().unchecked_ref()));

            mouse_closures.push(onmousedown);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmousehold = Closure::wrap(Box::new(move |e: MouseEvent| {
                if !hold_start.get() { return; }
                event_buffer.borrow_mut().push(Event::Mouse {
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
                hold_start.set(false);
                event_buffer.borrow_mut().push(Event::Mouse {
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
                let key_str = e.key();
                web_sys::console::log_1(&format!("keydown key: {}", key_str).into());
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
                    event_buffer.borrow_mut().push(Event::Key(key));
                } else if key_str.len() == 1 {
                    event_buffer.borrow_mut().push(Event::Char(key_str[0] as char));
                };
            }) as Box<dyn Fn(KeyboardEvent)>);
            input.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));

            keyboard_closures.push(onkeydown);
        }

        {
            let target: &EventTarget = input.as_ref();
            let event_buffer = event_buffer.clone();
            let oncompositionend = Closure::wrap(Box::new(move |e: CompositionEvent| {
                let data = e.data().unwrap();
                web_sys::console::log_1(&format!("compositionend data: {}", data).into());

                let mut event_buffer = event_buffer.borrow_mut();
                for ch in data.chars() {
                    event_buffer.push(Event::Char(ch));
                }
            }) as Box<dyn Fn(CompositionEvent)>);
            target.add_event_listener_with_callback("compositionend", oncompositionend.as_ref().unchecked_ref())?;

            composition_closures.push(oncompositionend);
        }

        Ok(Box::new(Self {
            document,
            console,
            _input: input,
            font_width: width,
            font_height: height,
            event_buffer,
            color: Cell::new(ColorPair {
                front: Color::TerminalDefault,
                back: Color::TerminalDefault,
            }),
            cur_bg_color: RefCell::default(),
            color_cache: RefCell::default(),
            effect: Cell::new(Effect::Simple),
            _closures: closures,
            _mouse_closures: mouse_closures,
            _touch_closures: touch_closures,
            _keyboard_closures: keyboard_closures,
            _composition_closures: composition_closures,
        }))
    }

    #[inline]
    fn clear_with(&self, color: String) {
        self.console.style().set_property("background-color", &color).unwrap();
        *self.cur_bg_color.borrow_mut() = color;

        while self.console.child_element_count() > 1 {
            self.console.remove_child(&self.console.last_child().unwrap()).unwrap();
        }
    }
}

impl backend::Backend for Backend {
    fn poll_event(&mut self) -> Option<Event> {
        self.event_buffer.borrow_mut().pop()
    }

    fn clear(&self, color: Color) {
        self.clear_with(color_to_html(color));
    }

    fn print_at(&self, pos: Vec2, text: &str) {
        let color_cache = self.color_cache.borrow();

        // Don't need draw bg
        if text.as_bytes().into_iter().all(|b| *b == b' ') && color_cache.bg_color.eq(&*self.cur_bg_color.borrow()) {
            return;
        }

        let x = pos.x * self.font_width;
        let y = pos.y * self.font_height;

        let span: HtmlSpanElement = self.document.create_element("span").expect("create_element").unchecked_into();
        span.style().set_property("position", "absolute").unwrap();
        span.style().set_property("color", &color_cache.color).unwrap();
        span.style().set_property("background-color", &color_cache.bg_color).unwrap();
        span.style().set_property("top", y.to_string().as_str()).unwrap();
        span.style().set_property("left", x.to_string().as_str()).unwrap();

        span.set_inner_text(text);
        //TODO: use effect

        self.console.append_child(&span).unwrap();
    }

    fn refresh(&mut self) {
        //TODO: what to do? double buffering?
    }

    fn screen_size(&self) -> Vec2 {
        Vec2::new(
            self.console.offset_width() as usize / self.font_width,
            self.console.offset_height() as usize / self.font_height,
        )
    }

    fn finish(&mut self) {}

    fn has_colors(&self) -> bool {
        true
    }

    fn set_color(&self, colors: ColorPair) -> ColorPair {
        let old = self.color.replace(colors);

        let mut color_cache = self.color_cache.borrow_mut();

        color_cache.color = color_to_html(colors.front);
        color_cache.bg_color = color_to_html(colors.back);

        old
    }

    fn set_effect(&self, effect: Effect) {
        self.effect.set(effect);
    }

    fn unset_effect(&self, _effect: Effect) {
        self.effect.set(Effect::Simple);
    }
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

