#[cfg(feature = "worker")]
mod backend;
#[cfg(feature = "worker-2d")]
mod backend_2d;
#[cfg(feature = "canvas")]
mod canvas;
mod event_handler;
#[cfg(any(feature = "worker", feature = "worker-2d"))]
mod frontend;
#[cfg(any(feature = "worker", feature = "worker-2d"))]
mod shared;

#[cfg(feature = "worker")]
pub use self::backend::Backend as WorkerBackend;

#[cfg(any(feature = "worker", feature = "worker-2d"))]
pub use self::{frontend::Frontend as WorkerFrontend, shared::Shared as WorkerShared};

#[cfg(feature = "worker-2d")]
pub use self::backend_2d::Backend as Worker2dBackend;

#[cfg(feature = "canvas")]
pub use self::canvas::Backend as CanvasBackend;
