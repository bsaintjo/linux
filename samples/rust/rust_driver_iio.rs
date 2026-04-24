// SPDX-License-Identifier: GPL-2.0

//! Rust IIO driver sample.

use kernel::{
    // c_str,
    faux,
    iio,
    // new_mutex,
    prelude::*,
    sync::Mutex,
};

module! {
    type: DummyModule,
    name: "rust_driver_iio",
    authors: ["Brandon Saint-John"],
    description: "Rust dummy driver sample",
    license: "GPL",
}

struct DummyModule {
    _fdev: faux::Registration,
    _indio_dev: iio::Registration<DummyDevice>,
}

impl kernel::Module for DummyModule {
    fn init(_module: &'static ThisModule) -> kernel::error::Result<Self> {
        todo!()
    }
}

#[pin_data]
struct DummyDevice {
    #[pin]
    _st: Mutex<DummyState>,
}

struct DummyState {
    _dac_val: i32,
}