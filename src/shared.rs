use std::collections::VecDeque;
use std::sync::Mutex;

use crate::event_handler::WasmEvent;

pub struct Shared {
    pub(crate) event_buffer: Mutex<VecDeque<WasmEvent>>,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            event_buffer: Mutex::new(VecDeque::new()),
        }
    }
}
