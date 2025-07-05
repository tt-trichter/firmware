use defmt::debug;
use embassy_time::Timer;
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};

use crate::output_from_pin;

pub struct IndicatorLights {
    rgb_led_red: Output<'static>,
    rgb_led_green: Output<'static>,
    rgb_led_blue: Output<'static>,
    onboard_led: Output<'static>,
}

impl IndicatorLights {
    pub fn new(
        pin_rgb_red: impl OutputPin + 'static,
        pin_rgb_green: impl OutputPin + 'static,
        pin_rgb_blue: impl OutputPin + 'static,
        pin_onboard: impl OutputPin + 'static,
    ) -> Self {
        let rgb_led_red = Output::new(pin_rgb_red, Level::High, OutputConfig::default());
        let rgb_led_green = Output::new(pin_rgb_green, Level::High, OutputConfig::default());
        let rgb_led_blue = Output::new(pin_rgb_blue, Level::High, OutputConfig::default());

        let onboard_led = output_from_pin(pin_onboard);

        debug!("initialising indicator lights...");

        Self {
            rgb_led_red,
            rgb_led_green,
            rgb_led_blue,
            onboard_led,
        }
    }

    pub async fn initialization_complete(&mut self) {
        for _ in 0..3 {
            self.onboard_led.set_high();
            Timer::after_millis(100).await;
            self.onboard_led.set_low();
            Timer::after_millis(100).await;
        }
    }

    pub fn error(&mut self) {
        self.onboard_led.set_high();
    }

    pub fn await_session(&mut self) {
        self.rgb_led_blue.set_low();

        self.rgb_led_green.set_high();
        self.rgb_led_red.set_high();
    }

    pub fn startup_session(&mut self) {
        self.rgb_led_red.set_low();

        self.rgb_led_green.set_high();
        self.rgb_led_blue.set_high();
    }

    pub fn start_session(&mut self) {
        self.rgb_led_green.set_low();

        self.rgb_led_blue.set_high();
        self.rgb_led_red.set_high();
    }

    pub fn stop_session(&mut self) {
        self.rgb_led_green.set_high();

        self.rgb_led_blue.set_high();
        self.rgb_led_red.set_high();
    }
}
