use defmt::{debug, info};
use esp_hal::{
    clock::CpuClock,
    gpio::InputPin,
    peripherals::{self, Peripherals},
    timer::systimer::Alarm,
};
use esp_wifi::EspWifiController;

use crate::{driver::sensor::SensorDriver, wifi::WifiManager};

pub struct System<'a> {
    pub wifi: Option<WifiManager<'a>>,
    pub sensor: Option<SensorDriver<'a>>,
}

impl System<'_> {
    pub fn init_peripherals() -> Peripherals {
        let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        esp_hal::init(config)
    }

    pub fn builder(alarm: Alarm<'static>) -> SystemBuilder {
        SystemBuilder::new(alarm)
    }
}

pub struct SystemBuilder {
    wifi: Option<WifiManager<'static>>,
    sensor: Option<SensorDriver<'static>>,
}

impl SystemBuilder {
    pub fn new(alarm: Alarm<'static>) -> Self {
        esp_alloc::heap_allocator!(size: 72 * 1024);

        esp_hal_embassy::init(alarm);
        debug!("embassy initialized");

        Self {
            wifi: None,
            sensor: None,
        }
    }

    pub fn with_wifi(
        mut self,
        wifi_init: &'static EspWifiController<'static>,
        wifi: peripherals::WIFI<'static>,
        bt: peripherals::BT<'static>,
    ) -> Self {
        esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 64 * 1024);
        self.wifi = Some(WifiManager::init(wifi_init, wifi, bt));

        self
    }

    pub fn with_sensor(mut self, pin: impl InputPin + 'static) -> Self {
        self.sensor = Some(SensorDriver::new(pin));
        self
    }

    pub fn build(self) -> System<'static> {
        info!("system initialized");
        System {
            wifi: self.wifi,
            sensor: self.sensor,
        }
    }
}
