use arraydeque::ArrayDeque;
use arraydeque::CapacityError;

use std::cell::UnsafeCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Poll};
use web_sys::Window;

use crate::event_handler::WasmEvent;

const EVENT_CAPACITY: usize = 2048;
type EventBufferType = UnsafeCell<ArrayDeque<[WasmEvent; EVENT_CAPACITY]>>;

pub struct Shared {
    is_busy: AtomicBool,
    event_buffer: EventBufferType,
}

unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

#[derive(Debug)]
pub struct BusyError;

pub struct SharedLockGuard<'a>(&'a Shared);

impl<'a> Drop for SharedLockGuard<'a> {
    fn drop(&mut self) {
        self.0.is_busy.store(false, Ordering::Relaxed);
    }
}

impl<'a> SharedLockGuard<'a> {
    #[inline]
    pub(crate) fn enqueue(&mut self, e: WasmEvent) -> Result<(), CapacityError<WasmEvent>> {
        unsafe { (*self.0.event_buffer.get()).push_back(e) }
    }

    #[inline]
    pub(crate) fn dequeue(&mut self) -> Option<WasmEvent> {
        unsafe { (*self.0.event_buffer.get()).pop_front() }
    }
}

impl Shared {
    pub(crate) fn try_lock(&self) -> Result<SharedLockGuard, BusyError> {
        if self
            .is_busy
            .compare_and_swap(false, true, Ordering::Relaxed)
        {
            return Err(BusyError);
        }

        Ok(SharedLockGuard(self))
    }
}
