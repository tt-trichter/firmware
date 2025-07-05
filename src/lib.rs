#![no_std]
#![feature(type_alias_impl_trait)]

use alloc::fmt;
use driver::indicator_lights::IndicatorLights;
use esp_hal::gpio::{Output, OutputConfig, OutputPin};

pub mod config;
pub mod driver;
pub mod system;
pub mod wifi;

extern crate alloc;

pub fn output_from_pin<'p>(p: impl OutputPin + 'p) -> Output<'p> {
    Output::new(p, esp_hal::gpio::Level::Low, OutputConfig::default())
}

pub fn ok_or_panic<T, E: fmt::Debug>(result: Result<T, E>, indicators: &mut IndicatorLights) -> T {
    match result {
        Ok(val) => val,
        Err(e) => {
            indicators.error();
            panic!("result was not ok: {:?}", e);
        }
    }
}

#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}
