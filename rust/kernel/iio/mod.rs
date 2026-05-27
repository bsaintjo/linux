#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(missing_docs)]
#![allow(unused_imports)]
use core::{
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::{addr_of_mut, NonNull},
};

use crate::{
    device,
    error::{to_result, VTABLE_DEFAULT_ERROR},
    prelude::*,
    str::CStr,
    sync::aref::ARef,
    ThisModule,
};

pub struct Device<T: Driver> {
    indio_dev: NonNull<bindings::iio_dev>,
    _priv: PhantomData<T>,
}

unsafe impl<T: Send + Sync + Driver> Send for Device<T> {}
unsafe impl<T: Send + Sync + Driver> Sync for Device<T> {}

pub struct DeviceRef(NonNull<bindings::iio_dev>);

impl DeviceRef {
    pub(crate) fn inner(&self) -> NonNull<bindings::iio_dev> {
        self.0
    }
}

impl<T: Driver> Device<T> {
    fn from_raw(indio_dev: *mut bindings::iio_dev) -> Self {
        let indio_dev = unsafe { NonNull::new_unchecked(indio_dev) };
        Self {
            indio_dev,
            _priv: PhantomData,
        }
    }

    pub(crate) fn dev_ref(&self) -> DeviceRef {
        DeviceRef(self.indio_dev)
    }

    // Useful when state needs to be initialized based on the iio device
    // Such as with triggers
    fn register_with(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        data: impl PinInit<T::Data>,
        with_device: impl FnOnce(DeviceRef) -> Result<(), Error>,
    ) -> Result<Self> {
        let sizeof_priv = mem::size_of::<T::Data>();
        let indio_dev = NonNull::new(unsafe {
            bindings::iio_device_alloc(parent.as_raw(), sizeof_priv as i32)
        })
        .ok_or(ENOMEM)?;
        let dev_ref = DeviceRef(indio_dev);
        with_device(dev_ref)?;
        let private: *mut T::Data =
            unsafe { bindings::iio_priv(indio_dev.as_ptr()) } as *mut T::Data;

        // SAFETY:
        // - *iio_device_alloc succeeded, so private is guaranteed to be a pointer to unitialized memory
        // - The uninitialized memory is is guaranteed to fit T::Data
        // - TODO: private is aligned for DMA, is this still correct?
        unsafe {
            data.__pinned_init(private).inspect_err(|_| {
                bindings::iio_device_free(indio_dev.as_ptr());
            })?;
        }
        unsafe {
            addr_of_mut!((*indio_dev.as_ptr()).name).write(options.name.as_char_ptr());
            addr_of_mut!((*indio_dev.as_ptr()).modes).write(options.modes as i32);
            addr_of_mut!((*indio_dev.as_ptr()).channels)
                .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec);
            addr_of_mut!((*indio_dev.as_ptr()).num_channels).write(T::CHANNELS.len() as ffi::c_int);
            addr_of_mut!((*indio_dev.as_ptr()).info)
                .write(IioVTableAdapter::<T>::build() as *const bindings::iio_info);
        }
        unsafe {
            to_result(bindings::__iio_device_register(
                indio_dev.as_ptr(),
                module.as_ptr(),
            ))?
        };
        Ok(Self {
            indio_dev,
            _priv: PhantomData,
        })
    }
}

impl<T: Driver> Drop for Device<T> {
    fn drop(&mut self) {
        unsafe { bindings::iio_device_unregister(self.indio_dev.as_ptr()) };
        unsafe { bindings::iio_device_free(self.indio_dev.as_ptr()) };
    }
}

pub struct RegistrationOptions {
    pub name: &'static CStr,
    pub modes: Mode,
}

#[repr(u32)]
pub enum Mode {
    Direct = bindings::INDIO_DIRECT_MODE,
}

pub struct Channel;

#[vtable]
pub trait Driver: Sized {
    const CHANNELS: &'static [Channel];
    type Data: Send + Sync;

    fn read_raw(device: &Device<Self>) {
        build_error!(VTABLE_DEFAULT_ERROR)
    }

    fn write_raw(device: &Device<Self>) -> Result {
        build_error!(VTABLE_DEFAULT_ERROR)
    }
}

pub struct IioVTableAdapter<T: Driver>(PhantomData<T>);

impl<T: Driver> IioVTableAdapter<T> {
    unsafe extern "C" fn read_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: *mut ffi::c_int,
        // Ignore for now
        _val2: *mut ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        todo!()
    }
    unsafe extern "C" fn write_raw(
        indio_dev: *mut bindings::iio_dev,
        iio_chan_spec: *const bindings::iio_chan_spec,
        val: ffi::c_int,
        _val2: ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        todo!()
    }

    const VTABLE: bindings::iio_info = bindings::iio_info {
        read_raw: Some(Self::read_raw),
        write_raw: Some(Self::write_raw),
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };

    const fn build() -> &'static bindings::iio_info {
        &Self::VTABLE
    }
}
