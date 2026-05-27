// SPDX-License-Identifier: GPL-2.0

//! Rust IIO driver sample.

#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(unused_imports)]

use kernel::{
    faux,
    iio::{
        self,
        channels::{self, Channel, ChannelType, SensorData, Specification},
        Device, DeviceRef, Driver, RegistrationOptions,
    },
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
    fdev: faux::Registration,
    indio_dev: iio::Device<DummyDevice>,
}

impl kernel::Module for DummyModule {
    fn init(module: &'static ThisModule) -> kernel::error::Result<Self> {
        pr_info!("Initialising Rust IIO Dummy Driver\n");

        let fdev = faux::Registration::new(c"rust-faux-parent", None)?;
        let dev = ARef::from(fdev.as_ref());

        let options = RegistrationOptions {
            name: c"rust-iio-driver",
            modes: iio::Mode::Direct,
        };

        let indio_dev =
            iio::Device::<DummyDevice>::register_with(dev, module, options, DummyState::init)?;
        Ok(Self { fdev, indio_dev })
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
            dac_val <- new_mutex!(717)
        })
    }
}

#[vtable]
impl Driver for DummyDevice {
    type Data = DummyState;

    const CHANNELS: &'static [Channel] = &[Specification::new(ChannelType::Voltage)
        .info_mask_separate(channels::RAW.or(channels::OFFSET).or(channels::SCALE))
        .channel_idx(0)
        .as_output()
        .as_channel()];

    fn read_raw(indio_dev: &Device<Self>, channel: &Specification) -> Result<SensorData> {
        if matches!(channel.channel_type(), ChannelType::Voltage) {
            let val = indio_dev.data().dac_val.lock().clone();
            Ok(SensorData::Int(val))
        } else {
            Err(EINVAL)
        }
    }

    fn write_raw(_device: &iio::Device<Self>) -> Result {
        Err(EINVAL)
    }
}
