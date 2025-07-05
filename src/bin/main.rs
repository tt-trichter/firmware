#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_hal::{
    rng::Rng,
    timer::{systimer::SystemTimer, timg::TimerGroup},
};
use esp_wifi::{init, EspWifiController};
use trichter::{
    driver::{
        indicator_lights::IndicatorLights,
        sensor::{SessionResult, StartupWindow, RESULTS},
    },
    mk_static, ok_or_panic,
    system::System,
    wifi::SessionResultClient,
};
use {esp_backtrace as _, esp_println as _};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let peripherals = System::init_peripherals();
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let mut indicators = IndicatorLights::new(
        peripherals.GPIO46,
        peripherals.GPIO0,
        peripherals.GPIO45,
        peripherals.GPIO48,
    );

    let rng = Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = &*mk_static!(
        EspWifiController<'static>,
        init(timer1.timer0, rng, peripherals.RADIO_CLK).unwrap()
    );
    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    let mut system = System::builder(timer0.alarm0)
        .with_sensor(peripherals.GPIO9)
        .with_wifi(wifi_init, peripherals.WIFI, peripherals.BT)
        .build();

    let wifi = system.wifi.take().unwrap();

    indicators.initialization_complete().await;

    let stack = wifi.connect_to_hotspot(rng, spawner).await;

    let mut result_client = ok_or_panic(SessionResultClient::new(stack).await, &mut indicators);

    let mut sensor = system.sensor.take().expect("sensor was not initialized");
    let duration = Duration::from_secs(10);
    loop {
        info!("Waiting for session to start...");

        let res = sensor
            .mesaure_session(
                StartupWindow::default(),
                Duration::from_millis(100),
                &mut indicators,
            )
            .await;
        info!(
            "Measured for {}ms with a flow rate of {}L/min",
            res.rate,
            res.duration.as_millis()
        );
        ok_or_panic(result_client.publish_result(res).await, &mut indicators);
    }
}
