use crate::shared::Shared;
use cursive::backend;
use cursive::event::{Event, Key, MouseButton, MouseEvent as CursiveMouseEvent};
use cursive::theme::{BaseColor, Color, ColorPair, Effect};
use cursive::Vec2;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlDivElement, HtmlElement, HtmlInputElement};

use crate::event_handler::EventHandler;

pub struct Frontend {
    event_handler: EventHandler,
    shared: Arc<Shared>,
}

impl Drop for Frontend {
    fn drop(&mut self) {}
}

impl Frontend {
    pub fn update(&self) {
        let mut buffer = self.event_handler.event_buffer().borrow_mut();

        if buffer.len() == 0 {
            return;
        }

        if let Ok(mut shared) = self.shared.try_lock() {
            for e in buffer.drain(..) {
                shared.enqueue(e).unwrap();
            }
        }
    }

    pub fn init(
        container: HtmlDivElement,
        console: HtmlCanvasElement,
        input: HtmlInputElement,
        composition_text: HtmlElement,
        shared: Arc<Shared>,
    ) -> Result<Self, JsValue> {
        let event_handler = EventHandler::new(container, console, input, composition_text)?;

        Ok(Self {
            event_handler,
            shared,
        })
    }
}
