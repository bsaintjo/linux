// SPDX-License-Identifier: GPL-2.0

//! Rust IIO driver sample.

#![allow(dead_code)]
#![allow(unreachable_code)]

use kernel::{
    c_str, faux,
    iio::{self, DeviceRef, Driver, RegistrationOptions},
    new_mutex,
    prelude::*,
    sync::{aref::ARef, Mutex},
};

module! {
    type: DummyModule,
    name: "rust_driver_iio",
    authors: ["Brandon Saint-John"],
    description: "Rust dummy driver sample",
    license: "GPL",
}

struct DummyModule {
    indio_dev: iio::Device<DummyDevice>,
    fdev: faux::Registration,
}

impl kernel::Module for DummyModule {
    fn init(module: &'static ThisModule) -> kernel::error::Result<Self> {
        pr_info!("Initialising Rust IIO Dummy Driver\n");

        let fdev = faux::Registration::new(c_str!("rust-faux-parent"), None)?;
        let dev = ARef::from(fdev.as_ref());

        let options = RegistrationOptions {
            name: c_str!("rust-iio-driver"),
            modes: iio::Mode::Direct,
        };

        let indio_dev =
            iio::Device::<DummyDevice>::register_with(dev, module, options, DummyState::init)?;
        Ok(Self {
            fdev,
            indio_dev,
        })
    }
}

struct DummyDevice;

#[pin_data]
struct DummyState {
    #[pin]
    dac_val: Mutex<i32>,
}

impl DummyState {
    fn init(_dev: DeviceRef<'_>) -> impl PinInit<Self, Error> {
        try_pin_init!(Self {
            dac_val <- new_mutex!(0)
        })
    }
}

#[vtable]
impl Driver for DummyDevice {
    type Data = DummyState;

    fn read_raw(_device: &iio::Device<Self>) {
        todo!()
    }

    fn write_raw(_device: &iio::Device<Self>) -> Result {
        todo!()
    }
}
