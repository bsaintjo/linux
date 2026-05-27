#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(missing_docs)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
pub mod channels;

use core::{
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::{addr_of_mut, NonNull},
};

use crate::{
    device,
    error::{to_result, VTABLE_DEFAULT_ERROR},
    iio::channels::{Channel, SensorData, Simple, Specification},
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

pub struct DeviceRef<'a>(NonNull<bindings::iio_dev>, PhantomData<&'a ()>);

impl<'a> DeviceRef<'a> {
    pub(crate) fn inner(&self) -> NonNull<bindings::iio_dev> {
        self.0
    }
}

impl<T: Driver> Device<T> {
    // Useful when state needs to be initialized based on the iio device
    // Such as with triggers
    pub fn register_with<P>(
        parent: ARef<device::Device>,
        module: &'static ThisModule,
        options: RegistrationOptions,
        with_device: impl FnOnce(DeviceRef<'_>) -> P,
    ) -> Result<Self>
    where
        P: PinInit<T::Data, Error>, // impl not allowed in return type so use a where clause
    {
        let sizeof_priv = mem::size_of::<T::Data>();
        let indio_dev = NonNull::new(unsafe {
            bindings::iio_device_alloc(parent.as_raw(), sizeof_priv as i32)
        })
        .ok_or(ENOMEM)?;
        let dev_ref = DeviceRef(indio_dev, PhantomData);
        let data = with_device(dev_ref);
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
            let dev = indio_dev.as_ptr();
            (&raw mut (*dev).name).write(options.name.as_char_ptr());
            (&raw mut (*dev).modes).write(options.modes as i32);
            (&raw mut (*dev).channels)
                .write(T::CHANNELS.as_ptr() as *const bindings::iio_chan_spec);
            (&raw mut (*dev).num_channels).write(T::CHANNELS.len() as ffi::c_int);
            (&raw mut (*dev).info)
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

    pub fn data(&self) -> Pin<&T::Data> {
        let data: *const T::Data = unsafe { bindings::iio_priv(self.indio_dev.as_ptr()) }.cast();
        let data = unsafe { &*data };
        let data = unsafe { Pin::new_unchecked(data) };
        data
    }

    pub fn data_mut(&mut self) -> Pin<&mut T::Data> {
        let data: *mut T::Data = unsafe { bindings::iio_priv(self.indio_dev.as_ptr()) }.cast();
        let data = unsafe { &mut *data };
        let data = unsafe { Pin::new_unchecked(data) };
        data
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

#[vtable]
pub trait Driver: Sized {
    const CHANNELS: &'static [Channel] = &[];
    type Data: Send + Sync;

    fn read_raw(data: &Device<Self>, channel: &Specification) -> Result<SensorData> {
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
        _val2: *mut ffi::c_int,
        _mask: isize,
    ) -> ffi::c_int {
        // Manually drop so that Device::drop doesn't unregister at the end of the scope
        // Alternative is to use something like DeviceRef
        let indio_dev: ManuallyDrop<Device<T>> = ManuallyDrop::new(Device {
            indio_dev: NonNull::new(indio_dev).unwrap(),
            _priv: PhantomData,
        });

        let channel = unsafe { &*iio_chan_spec.cast::<Specification<Simple>>() };

        match T::read_raw(&indio_dev, channel) {
            Ok(sdata) => {
                pr_emerg!("read_raw successfully, sending data");
                match &sdata {
                    SensorData::Int(inner) => unsafe {
                        let val = val as *mut MaybeUninit<i32>;
                        (*val).write(*inner);
                    },
                }
                sdata.sensor_value() as ffi::c_int
            }
            Err(e) => e.to_errno(),
        }
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
