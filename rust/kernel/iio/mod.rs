#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(missing_docs)]
#![allow(unreachable_code)]

use crate::build_error;
use crate::error::VTABLE_DEFAULT_ERROR;
use crate::prelude::*;
use core::{marker::PhantomData, pin::Pin, ptr::NonNull};

use pin_init::{pin_data, PinInit};

use crate::{device, error::Error, sync::aref::ARef, ThisModule};

#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Device<T: Driver> {
    #[pin]
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

unsafe impl<T: Send + Sync + Driver> Send for Device<T> {}
unsafe impl<T: Send + Sync + Driver> Sync for Device<T> {}

impl<T: Driver> Device<T> {
    pub fn register(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        data: impl PinInit<T::Data, Error>,
    ) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            indio_dev: todo!(),
            _priv: todo!(),
        })
    }
}

#[pinned_drop]
impl<T: Driver> PinnedDrop for Device<T> {
    fn drop(self: Pin<&mut Self>) {
        unsafe { bindings::iio_device_unregister(self.indio_dev.as_ptr()) };
        unsafe { bindings::iio_device_free(self.indio_dev.as_ptr()) };
    }
}

#[vtable]
pub trait Driver: Sized {
    type Data: Send + Sync;

    fn read_raw(_data: Pin<&Self::Data>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(_data: Pin<&mut Self::Data>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}
