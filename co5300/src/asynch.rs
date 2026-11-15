//! Async driver on [`AsyncInterface`](crate::AsyncInterface).
//!
//! Mirrors the blocking [`Co5300`](crate::Co5300); see there for the datasheet references.
//! Both are generated from the same source.

crate::driver::define_driver!(
    crate::AsyncInterface,
    embedded_hal_async::delay::DelayNs,
    "Async CO5300 driver.",
    [async],
    [.await]
);
