// SPDX-License-Identifier: GPL-2.0

//! Industrial I/O Driver subsystem
use core::{marker::PhantomData, ptr::NonNull};

use macros::vtable;
use pin_init::pin_data;

use crate::{device::Device, prelude::*, ThisModule};

/// TODO Registration docs
#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Registration<T> {
    #[pin]
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

// TODO Safety
unsafe impl<T: Send + Sync> Send for Registration<T> {}
unsafe impl<T: Send + Sync> Sync for Registration<T> {}

impl<T: Driver> Registration<T> {

    /// TODO
    pub fn new(_dev: &Device, _module: &'static ThisModule) -> Result<Self> {
        todo!()
    }
}

#[pinned_drop]
impl<T> PinnedDrop for Registration<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe {
            bindings::iio_device_unregister(self.indio_dev.as_ptr());
            bindings::iio_device_free(self.indio_dev.as_ptr());
        }
    }
}

/// TODO
#[vtable]
pub trait Driver {}
