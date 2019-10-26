use cursive::event::{Event, Key, MouseButton, MouseEvent as CursiveMouseEvent};
use cursive::Vec2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
#[cfg(feature = "composition")]
use web_sys::{CompositionEvent, EventTarget};
use web_sys::{
    HtmlCanvasElement, HtmlDivElement, HtmlElement, HtmlInputElement, KeyboardEvent, MouseEvent,
    TouchEvent,
};

#[derive(Copy, Clone, Debug)]
pub enum WasmEvent {
    MouseDown(Vec2, MouseButton),
    MouseMove(Vec2, MouseButton),
    MouseUp(Vec2, MouseButton),
    // TODO: wheel event
    Key(Key),
    Char(char),
    Resize(u32, u32),
}

impl WasmEvent {
    pub fn into_event(self, font_width: usize, font_height: usize) -> Event {
        match self {
            WasmEvent::MouseDown(pos, btn) => Event::Mouse {
                position: pos.map_x(|x| x / font_width).map_y(|y| y / font_height),
                offset: Vec2::new(0, 0),
                event: CursiveMouseEvent::Press(btn),
            },
            WasmEvent::MouseMove(pos, btn) => Event::Mouse {
                position: pos.map_x(|x| x / font_width).map_y(|y| y / font_height),
                offset: Vec2::new(0, 0),
                event: CursiveMouseEvent::Hold(btn),
            },
            WasmEvent::MouseUp(pos, btn) => Event::Mouse {
                position: pos.map_x(|x| x / font_width).map_y(|y| y / font_height),
                offset: Vec2::new(0, 0),
                event: CursiveMouseEvent::Release(btn),
            },
            WasmEvent::Key(key) => Event::Key(key),
            WasmEvent::Char(ch) => Event::Char(ch),
            WasmEvent::Resize(_, _) => Event::WindowResize,
        }
    }
}

pub struct EventHandler {
    event_buffer: Rc<RefCell<Vec<WasmEvent>>>,
    container: HtmlDivElement,
    console: HtmlCanvasElement,
    input: HtmlInputElement,

    _closures: Vec<Closure<dyn Fn()>>,
    _mouse_closures: Vec<Closure<dyn Fn(MouseEvent)>>,
    _touch_closures: Vec<Closure<dyn Fn(TouchEvent)>>,
    _keyboard_closures: Vec<Closure<dyn Fn(KeyboardEvent)>>,
    #[cfg(feature = "composition")]
    on_composition_end: Closure<dyn Fn(CompositionEvent)>,
    #[cfg(feature = "composition")]
    on_composition_update: Closure<dyn Fn(CompositionEvent)>,
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        self.console.set_onresize(None);
        self.container.set_onmousedown(None);
        self.container.set_onmousemove(None);
        self.container.set_onmouseup(None);
        self.container.set_ontouchstart(None);
        self.container.set_ontouchmove(None);
        self.container.set_ontouchend(None);
        self.input.set_onkeydown(None);

        #[cfg(feature = "composition")]
        {
            let target: &EventTarget = self.input.as_ref();
            target
                .remove_event_listener_with_callback(
                    "compositionend",
                    self.on_composition_end.as_ref().unchecked_ref(),
                )
                .ok();
            target
                .remove_event_listener_with_callback(
                    "compositionupdate",
                    self.on_composition_update.as_ref().unchecked_ref(),
                )
                .ok();
        }
    }
}

impl EventHandler {
    pub fn event_buffer(&self) -> &RefCell<Vec<WasmEvent>> {
        &self.event_buffer
    }

    pub fn new(
        container: HtmlDivElement,
        console: HtmlCanvasElement,
        input: HtmlInputElement,
        composition_text: HtmlElement,
    ) -> Result<Self, JsValue> {
        let mut closures = Vec::with_capacity(1);
        let mut mouse_closures = Vec::with_capacity(3);
        let mut touch_closures = Vec::with_capacity(3);
        let mut keyboard_closures = Vec::with_capacity(1);
        let event_buffer = Rc::new(RefCell::new(Vec::with_capacity(300)));
        let hold_start = Rc::new(Cell::new(false));

        {
            use web_sys::Element;
            let console2 = console.clone();
            let event_buffer = event_buffer.clone();
            let onresize = Closure::wrap(Box::new(move || {
                let client: &Element = console2.as_ref();
                let width = client.client_width() as u32;
                let height = client.client_height() as u32;
                console2.set_width(width);
                console2.set_height(height);
                event_buffer
                    .borrow_mut()
                    .push(WasmEvent::Resize(width, height));
            }) as Box<dyn Fn()>);
            console.set_onresize(Some(onresize.as_ref().unchecked_ref()));

            closures.push(onresize);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmousedown = Closure::wrap(Box::new(move |e: MouseEvent| {
                hold_start.set(true);
                event_buffer.borrow_mut().push(WasmEvent::MouseDown(
                    Vec2::new(e.x() as usize, e.y() as usize),
                    get_mouse_botton(&e),
                ));
            }) as Box<dyn Fn(MouseEvent)>);
            container.set_onmousedown(Some(onmousedown.as_ref().unchecked_ref()));

            mouse_closures.push(onmousedown);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmousehold = Closure::wrap(Box::new(move |e: MouseEvent| {
                if !hold_start.get() {
                    return;
                }
                event_buffer.borrow_mut().push(WasmEvent::MouseMove(
                    Vec2::new(e.x() as usize, e.y() as usize),
                    get_mouse_botton(&e),
                ));
            }) as Box<dyn Fn(MouseEvent)>);
            container.set_onmousemove(Some(onmousehold.as_ref().unchecked_ref()));

            mouse_closures.push(onmousehold);
        }

        {
            let hold_start = hold_start.clone();
            let event_buffer = event_buffer.clone();
            let onmouseup = Closure::wrap(Box::new(move |e: MouseEvent| {
                hold_start.set(false);
                event_buffer.borrow_mut().push(WasmEvent::MouseUp(
                    Vec2::new(e.x() as usize, e.y() as usize),
                    get_mouse_botton(&e),
                ));
            }) as Box<dyn Fn(MouseEvent)>);
            container.set_onmouseup(Some(onmouseup.as_ref().unchecked_ref()));

            mouse_closures.push(onmouseup);
        }

        {
            let event_buffer = event_buffer.clone();
            let input2 = input.clone();
            let onkeydown = Closure::wrap(Box::new(move |e: KeyboardEvent| {
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
                    event_buffer.borrow_mut().push(WasmEvent::Key(key));
                } else if key_str.len() == 1 {
                    event_buffer
                        .borrow_mut()
                        .push(WasmEvent::Char(key_str[0] as char));
                };

                input2.set_value("");
            }) as Box<dyn Fn(KeyboardEvent)>);
            input.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));

            keyboard_closures.push(onkeydown);
        }

        {
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

                let touch = e.touches().get(0).unwrap();
                hold_start.set(true);
                event_buffer.borrow_mut().push(WasmEvent::MouseDown(
                    Vec2::new(touch.client_x() as usize, touch.client_y() as usize),
                    MouseButton::Left,
                ));
            }) as Box<dyn Fn(TouchEvent)>);
            container.set_ontouchstart(Some(ontouchstart.as_ref().unchecked_ref()));

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
                event_buffer.borrow_mut().push(WasmEvent::MouseMove(
                    Vec2::new(touch.client_x() as usize, touch.client_y() as usize),
                    MouseButton::Left,
                ));
            }) as Box<dyn Fn(TouchEvent)>);
            container.set_ontouchmove(Some(ontouchmove.as_ref().unchecked_ref()));

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
                hold_start.set(false);
                event_buffer.borrow_mut().push(WasmEvent::MouseUp(
                    Vec2::new(touch.client_x() as usize, touch.client_y() as usize),
                    MouseButton::Left,
                ));
            }) as Box<dyn Fn(TouchEvent)>);
            container.set_ontouchend(Some(ontouchend.as_ref().unchecked_ref()));

            touch_closures.push(ontouchend);
        }
        #[cfg(feature = "composition")]
        let on_composition_update = {
            let target: &EventTarget = input.as_ref();
            let composition_text = composition_text.clone();
            let on_composition_update = Closure::wrap(Box::new(move |e: CompositionEvent| {
                let data = e.data().unwrap();

                log::trace!("compositionupdate: [{}]", data);

                composition_text.set_inner_text(&data);
            })
                as Box<dyn Fn(CompositionEvent)>);
            target.add_event_listener_with_callback(
                "compositionupdate",
                on_composition_update.as_ref().unchecked_ref(),
            )?;

            on_composition_update
        };

        #[cfg(feature = "composition")]
        let on_composition_end = {
            let target: &EventTarget = input.as_ref();
            let event_buffer = event_buffer.clone();
            let composition_text = composition_text.clone();
            let on_composition_end = Closure::wrap(Box::new(move |e: CompositionEvent| {
                let data = e.data().unwrap();

                log::trace!("compositionend: [{}]", data);

                let mut event_buffer = event_buffer.borrow_mut();
                for ch in data.chars() {
                    event_buffer.push(WasmEvent::Char(ch));
                }

                composition_text.set_inner_text("");
            }) as Box<dyn Fn(CompositionEvent)>);
            target.add_event_listener_with_callback(
                "compositionend",
                on_composition_end.as_ref().unchecked_ref(),
            )?;

            on_composition_end
        };

        Ok(Self {
            event_buffer,
            container,
            console,
            input,
            #[cfg(feature = "composition")]
            on_composition_end,
            #[cfg(feature = "composition")]
            on_composition_update,
            _closures: closures,
            _keyboard_closures: keyboard_closures,
            _mouse_closures: mouse_closures,
            _touch_closures: touch_closures,
        })
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
