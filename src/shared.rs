use js_sys::Atomics;
use js_sys::Int32Array;
use js_sys::SharedArrayBuffer;

use crate::event_handler::WasmEvent;

const LOCK_INDEX: u32 = 0;
const DRAW_INDEX: u32 = 1;
const LENGTH_INDEX: u32 = 2;
const DATA_INDEX: u32 = 3;
use std::mem::size_of;
const WASM_EVENT_SIZE: usize = size_of::<WasmEvent>() / size_of::<i32>();

pub struct Shared {
    event_buffer: Int32Array,
}

impl Shared {
    pub fn new(event_buffer: &SharedArrayBuffer) -> Self {
        Self {
            event_buffer: Int32Array::new(event_buffer.as_ref()),
        }
    }

    pub fn try_push(&self, items: &mut Vec<WasmEvent>) -> bool {
        if Atomics::compare_exchange(self.event_buffer.as_ref(), LOCK_INDEX, 0, 1).unwrap() == 0 {
            items.reverse();
            use std::slice::from_raw_parts;
            let length = items.len() * WASM_EVENT_SIZE;

            let prev_length =
                Atomics::add(self.event_buffer.as_ref(), LENGTH_INDEX, length as i32).unwrap();

            log::debug!("prev_length: {}", prev_length);

            self.event_buffer.set(
                unsafe { Int32Array::view(from_raw_parts(items.as_ptr() as *const i32, length)) }
                    .as_ref(),
                DATA_INDEX + prev_length as u32,
            );
            Atomics::store(self.event_buffer.as_ref(), LOCK_INDEX, 0).unwrap();
            Atomics::notify(&self.event_buffer, LOCK_INDEX).unwrap();
            items.clear();

            true
        } else {
            false
        }
    }

    pub fn pop(&self, buf: &mut Vec<WasmEvent>) {
        while Atomics::compare_exchange(self.event_buffer.as_ref(), LOCK_INDEX, 0, 1).unwrap() != 0
        {
            Atomics::wait(&self.event_buffer, LOCK_INDEX, 1).unwrap();
        }

        let _ = Atomics::wait_with_timeout(&self.event_buffer, LOCK_INDEX, 0, 100.0);

        let length = Atomics::exchange(self.event_buffer.as_ref(), LENGTH_INDEX, 0).unwrap() as u32;
        let event_length = length as usize / WASM_EVENT_SIZE;

        if length != 0 {
            log::debug!("Detect event!");
            buf.reserve(event_length);
            self.event_buffer
                .slice(DATA_INDEX, DATA_INDEX + length)
                .copy_to(unsafe {
                    std::slice::from_raw_parts_mut(
                        buf.as_mut_ptr().add(buf.len()) as *mut i32,
                        length as usize,
                    )
                });
            unsafe {
                buf.set_len(buf.len() + event_length);
            }
            log::debug!("events: {:?}", buf);
        }

        Atomics::store(self.event_buffer.as_ref(), LOCK_INDEX, 0).unwrap();
    }
}
