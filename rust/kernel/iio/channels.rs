#![allow(dead_code)]
/// Channel
///
/// Invariants:
/// Differential channels must be indexed
/// Duplicate scan index
/// Cannot use labels and extended_name at the same time
/// Multi-long available scan masks not fully supported
use core::{
    fmt,
    marker::PhantomData,
    mem::{self, MaybeUninit},
};

use bindings::iio_chan_info_enum;

// TODO: Explore a more thickly wrapped Specification on top of this one
// This way, we can use pattern matching to make the Rust side more ergonomic
// Idea:
// struct Channel {
//   pub something: i32,
//   pub to_: &'static CStr,
//   pub match: bool,
//   pub onto: Option<u64>
//   spec: Specification,
// }
// then in read_raw/etc...
// fn read_raw(channel: &Channel, /* args */) {
//     // Example with let chains
//     if ChannelType::Voltage = chan.spec.channel_type() && let Some(x) = chan.onto && x > 12 {
//          /* Rust fun */
//     }
// }

#[repr(transparent)]
#[derive(Copy, Clone, Default)]
pub struct Specification<T = Simple> {
    spec: bindings::iio_chan_spec,
    _phantom: PhantomData<T>,
}

#[derive(Copy, Clone, Default)]
pub struct Simple;

#[derive(Copy, Clone, Default)]
pub struct Buffered;

#[derive(Copy, Clone, Default)]
pub struct Channel {
    inner: bindings::iio_chan_spec,
}

pub struct ChannelDefinition {
    pub ctype: ChannelType,
    pub output: bool,
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Channel")
            .field("channel", &self.inner.channel)
            .field("modified", &self.inner.modified())
            .field("indexed", &self.inner.indexed())
            .field("output", &self.inner.output())
            .field("differential", &self.inner.differential())
            .finish()
    }
}

impl<T> Specification<T> {
    pub const fn as_channel(&'static self) -> Channel {
        Channel { inner: self.spec }
    }

    pub fn definition(&self) -> ChannelDefinition {
        ChannelDefinition {
            ctype: unsafe { mem::transmute(self.spec.type_) },
            output: self.spec.output() == 1,
        }
    }
}

// Feature? Automatically infer return type via channel type?
impl Specification<Simple> {
    pub const fn new(channel_type: ChannelType) -> Self {
        // TODO Can iio_chan_spec initialized to zero?
        unsafe {
            Specification {
                spec: bindings::iio_chan_spec {
                    type_: channel_type as ffi::c_uint,
                    scan_index: -1,
                    ..MaybeUninit::zeroed().assume_init()
                },
                _phantom: PhantomData,
            }
        }
    }

    pub const fn scan_index(mut self, idx: i32) -> Self {
        self.spec.scan_index = idx;
        self
    }

    pub const fn channel_idx(mut self, idx: i32) -> Self {
        self.spec.channel = idx;
        self.as_indexed()
    }

    pub const fn channel_type(&self) -> ChannelType {
        unsafe { core::mem::transmute(self.spec.type_) }
    }

    // TODO pull from actual specification
    pub fn is_differential(&self) -> bool {
        true
    }

    pub const fn info_mask_separate(mut self, mask: Mask) -> Self {
        self.spec.info_mask_separate = mask.0 as usize;
        self
    }

    // TODO: Figure out a better way to do this
    // .output is a bitfield member of iio_chan_spec
    // bindgen outputs a helper set_output to make is easy to set
    // however, it implements it using generic functions and isn't const
    // The below is copying out the necessary parts from bindgen so I can set
    // output in a const context
    pub const fn as_output(self) -> Self {
        let bit_offset = 2usize;
        self.set_offset(bit_offset)
    }

    pub const fn as_differential(self) -> Self {
        let bit_offset = 3usize;
        self.set_offset(bit_offset)
    }

    pub const fn as_indexed(self) -> Self {
        let bit_offset = 1usize;
        self.set_offset(bit_offset)
    }

    pub const fn set_offset(mut self, bit_offset: usize) -> Self {
        // TODO: make bindgen output bitfield operations as const
        let bad: [u8; 1] = unsafe { mem::transmute(self.spec._bitfield_1) };
        let mask: u8 = bad[0] | (1 << bit_offset);
        self.spec._bitfield_1 = bindings::__BindgenBitfieldUnit::new([mask; 1]);
        self
    }
}

impl Specification<Buffered> {
    // TODO Can iio_chan_spec initialized to zero?
    pub const fn new_buffered(channel_type: ChannelType) -> Self {
        unsafe {
            Specification {
                spec: bindings::iio_chan_spec {
                    type_: channel_type as ffi::c_uint,
                    // scan_index: T::SCAN_TYPE.scan_index,
                    scan_index: 7,
                    ..MaybeUninit::zeroed().assume_init()
                },
                _phantom: PhantomData,
            }
        }
    }
}

#[macro_export]
macro_rules! concat_channels {
    ($a:expr, $b:expr) => {{
        let _a: &'static [$crate::iio::channels::Specification<$crate::iio::channels::Simple>] = $a;
        let _b: &'static [$crate::iio::channels::Specification<$crate::iio::channels::Buffered>] =
            $b;
        const LEN_A: usize = $a.len();
        const LEN_B: usize = $b.len();
        const LEN: usize = LEN_A + LEN_B;

        let mut result = unsafe { [core::mem::zeroed(); LEN] };
        let mut i = 0;
        while i < LEN_A {
            result[i] = $a[i].as_channel();
            i += 1;
        }
        let mut j = 0;
        while j < LEN_B {
            result[LEN_A + j] = $b[j].as_channel();
            j += 1;
        }
        result
    }};
}

#[derive(Clone, Copy, PartialEq)]
pub struct Mask(u32);

impl Mask {
    const fn new(chan_info: iio_chan_info_enum) -> Self {
        Mask(bits::bit_u32(chan_info))
    }

    pub const fn or(self, other: Mask) -> Self {
        Self(self.0 | other.0)
    }
}

pub const RAW: Mask = Mask::new(bindings::iio_chan_info_enum_IIO_CHAN_INFO_RAW);
pub const OFFSET: Mask = Mask::new(bindings::iio_chan_info_enum_IIO_CHAN_INFO_OFFSET);
pub const SCALE: Mask = Mask::new(bindings::iio_chan_info_enum_IIO_CHAN_INFO_SCALE);
pub const PROCESSED: Mask = Mask::new(bindings::iio_chan_info_enum_IIO_CHAN_INFO_PROCESSED);
pub const CALIBSCALE: Mask = Mask::new(bindings::iio_chan_info_enum_IIO_CHAN_INFO_CALIBSCALE);
pub const INT_TIME: Mask = Mask::new(bindings::iio_chan_info_enum_IIO_CHAN_INFO_INT_TIME);

#[derive(Debug)]
pub enum SensorData {
    Int(i32),
}

impl SensorData {
    pub fn int(x: i32) -> Self {
        SensorData::Int(x)
    }

    pub(crate) fn sensor_value(&self) -> SensorValue {
        match self {
            SensorData::Int(_) => SensorValue::Int,
        }
    }
}

/// Represents the return type
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum SensorValue {
    Int = bindings::IIO_VAL_INT,
    PlusMicro = bindings::IIO_VAL_INT_PLUS_MICRO,
    PlusNano = bindings::IIO_VAL_INT_PLUS_NANO,
    PlusMicroDb = bindings::IIO_VAL_INT_PLUS_MICRO_DB,
    IntMultiple = bindings::IIO_VAL_INT_MULTIPLE,
    Int64 = bindings::IIO_VAL_INT_64,
    Fractional = bindings::IIO_VAL_FRACTIONAL,
    FractionalLog2 = bindings::IIO_VAL_FRACTIONAL_LOG2,
    Char = bindings::IIO_VAL_CHAR,
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum ChannelType {
    Voltage = bindings::iio_chan_type_IIO_VOLTAGE,
    Current = bindings::iio_chan_type_IIO_CURRENT,
    Power = bindings::iio_chan_type_IIO_POWER,
    Accel = bindings::iio_chan_type_IIO_ACCEL,
    AnglVel = bindings::iio_chan_type_IIO_ANGL_VEL,
    Magn = bindings::iio_chan_type_IIO_MAGN,
    Light = bindings::iio_chan_type_IIO_LIGHT,
    Intensity = bindings::iio_chan_type_IIO_INTENSITY,
    Proximity = bindings::iio_chan_type_IIO_PROXIMITY,
    Temp = bindings::iio_chan_type_IIO_TEMP,
    Incli = bindings::iio_chan_type_IIO_INCLI,
    Rot = bindings::iio_chan_type_IIO_ROT,
    Angl = bindings::iio_chan_type_IIO_ANGL,
    Timestamp = bindings::iio_chan_type_IIO_TIMESTAMP,
    Capacitance = bindings::iio_chan_type_IIO_CAPACITANCE,
    AltVoltage = bindings::iio_chan_type_IIO_ALTVOLTAGE,
    Cct = bindings::iio_chan_type_IIO_CCT,
    Pressure = bindings::iio_chan_type_IIO_PRESSURE,
    HumidityRelative = bindings::iio_chan_type_IIO_HUMIDITYRELATIVE,
    Activity = bindings::iio_chan_type_IIO_ACTIVITY,
    Steps = bindings::iio_chan_type_IIO_STEPS,
    Energy = bindings::iio_chan_type_IIO_ENERGY,
    Distance = bindings::iio_chan_type_IIO_DISTANCE,
    Velocity = bindings::iio_chan_type_IIO_VELOCITY,
    Concentration = bindings::iio_chan_type_IIO_CONCENTRATION,
    Resistance = bindings::iio_chan_type_IIO_RESISTANCE,
    Ph = bindings::iio_chan_type_IIO_PH,
    UvIndex = bindings::iio_chan_type_IIO_UVINDEX,
    ElectricalConductivity = bindings::iio_chan_type_IIO_ELECTRICALCONDUCTIVITY,
    // TODO: From a talk a few years ago, it seemed like count sensors are moved out
    // So maybe don't allow for implmenting count channels
    // https://www.youtube.com/watch?v=644oH1FXdtE, 24:54, ABI 'mistakes'
    // Need to check on the status, and add the deprecated attribute until confirmed
    #[deprecated]
    Count = bindings::iio_chan_type_IIO_COUNT,
    Index = bindings::iio_chan_type_IIO_INDEX,
    Gravity = bindings::iio_chan_type_IIO_GRAVITY,
    PositionRelative = bindings::iio_chan_type_IIO_POSITIONRELATIVE,
    Phase = bindings::iio_chan_type_IIO_PHASE,
    MassConcentration = bindings::iio_chan_type_IIO_MASSCONCENTRATION,
    DeltaAngl = bindings::iio_chan_type_IIO_DELTA_ANGL,
    DeltaVelocit = bindings::iio_chan_type_IIO_DELTA_VELOCITY,
    ColorTemp = bindings::iio_chan_type_IIO_COLORTEMP,
    Chromaticity = bindings::iio_chan_type_IIO_CHROMATICITY,
    Attention = bindings::iio_chan_type_IIO_ATTENTION,
}

use macros::kunit_tests;

use crate::bits;
#[kunit_tests(rust_iio_channels)]
mod test {

    use super::*;

    #[test]
    fn test_spec_output() {
        let spec = Specification::new(ChannelType::Voltage).as_output();
        assert_eq!(spec.spec.output(), 1);
    }
}
