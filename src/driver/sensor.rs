use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use defmt::{debug, info};
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};
use esp_hal::gpio::{Event, Input, InputConfig, InputPin};

use super::indicator_lights::IndicatorLights;

pub struct SensorDriver<'d> {
    pub input: Input<'d>,
}

static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

impl<'d> SensorDriver<'d> {
    pub fn new(pin: impl InputPin + 'd) -> Self {
        let mut inp = Input::new(
            pin,
            InputConfig::default().with_pull(esp_hal::gpio::Pull::Up),
        );
        inp.listen(Event::RisingEdge);
        debug!("sensor driver initialized");
        SensorDriver { input: inp }
    }

    pub async fn mesaure_session(
        &mut self,
        startup_window: StartupWindow,
        idle_timeout: Duration,
        indicators: &mut IndicatorLights,
    ) -> SessionResult {
        let mut first: Instant;
        let mut last: Instant;
        self.input.clear_interrupt();
        loop {
            indicators.await_session();
            self.input.wait_for_rising_edge().await;
            indicators.startup_session();
            first = Instant::now();
            last = first;
            PULSE_COUNT.store(1, Ordering::Relaxed);

            let startup_deadline = first + startup_window.length;

            while Instant::now() < startup_deadline {
                let remaining = startup_deadline - Instant::now();
                match select(self.input.wait_for_rising_edge(), Timer::after(remaining)).await {
                    Either::First(_) => {
                        let now = Instant::now();
                        PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
                        last = now;
                    }
                    Either::Second(_) => {
                        info!(
                            "StartUp Window not fullfilled, received {} pulses in {} ms",
                            PULSE_COUNT.load(Ordering::Relaxed),
                            startup_window.length.as_millis()
                        );
                    }
                }
            }

            let seen = PULSE_COUNT.load(Ordering::Relaxed);
            if seen >= startup_window.pulses {
                break;
            }
        }
        indicators.start_session();
        loop {
            let edge_fut = self.input.wait_for_rising_edge();
            let timeout_fut = Timer::after(idle_timeout);

            match select(edge_fut, timeout_fut).await {
                Either::First(_) => {
                    PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
                    last = Instant::now();
                    self.input.clear_interrupt();
                }
                Either::Second(_) => break,
            }
        }
        indicators.stop_session();

        let pulses = PULSE_COUNT.load(Ordering::Relaxed);
        let duration = last - first;
        let rate = Self::pulses_to_flow(pulses, duration);
        info!(
            "Pulses: {}, Rate: {}, DurationMs: {}",
            pulses,
            rate,
            duration.as_millis(),
        );
        SessionResult::new(duration, rate)
    }

    pub async fn measure_duration(&mut self, duration: Duration) -> f32 {
        self.input.listen(Event::RisingEdge);
        PULSE_COUNT.store(0, Ordering::Relaxed);
        let start = Instant::now();
        while Instant::now() - start < duration {
            if self.input.is_interrupt_set() {
                PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
                self.input.clear_interrupt();
                self.input.listen(Event::RisingEdge);
            }
        }
        let pulses = PULSE_COUNT.load(Ordering::Relaxed);
        Self::pulses_to_flow(pulses, duration)
    }

    pub fn pulses_to_flow(pulses: u32, duration: Duration) -> f32 {
        let window_s = duration.as_micros() as f32 / 1_000_000.0;
        let pulses_per_sec = pulses as f32 / window_s;
        pulses_per_sec * 60.0 / 6.6
    }
}

pub struct StartupWindow {
    pub pulses: u32,
    pub length: Duration,
}

impl StartupWindow {
    pub fn new(pulses: u32, window_ms: u64) -> Self {
        Self {
            pulses,
            length: Duration::from_millis(window_ms),
        }
    }
}

impl Default for StartupWindow {
    fn default() -> Self {
        Self {
            pulses: 10,
            length: Duration::from_millis(100),
        }
    }
}

const HISTORY_SIZE: usize = 16;

pub struct SessionResult {
    pub duration: Duration,
    pub rate: f32,
    pub volume: f32,
}

impl SessionResult {
    pub fn new(duration: Duration, rate: f32) -> Self {
        let minutes = duration.as_millis() as f32 / 60_000.0;
        let volume = rate * minutes;

        Self {
            duration,
            rate,
            volume,
        }
    }
}

pub static RESULTS: Mutex<CriticalSectionRawMutex, Vec<SessionResult>> = Mutex::new(Vec::new());
