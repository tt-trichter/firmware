use crate::{
    config::{
        remote::{REMOTE_DEV_SECRET, REMOTE_ENDPOINT, REMOTE_ENDPOINT_STR, RESULT_RESOURCE},
        wifi::{PASSWORD, SSID},
    },
    driver::sensor::SessionResult,
    mk_static,
};
use alloc::format;
use defmt::{debug, error, info};
use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write as _;
use esp_hal::{
    peripherals::{self},
    rng::Rng,
};
use esp_wifi::{
    wifi::{
        ClientConfiguration, Configuration, Interfaces, WifiController, WifiDevice, WifiEvent,
        WifiState,
    },
    EspWifiController,
};

pub struct WifiManager<'d> {
    interfaces: Interfaces<'d>,
    wifi_controller: WifiController<'d>,
    // ble_controller: ExternalController<BleConnector<'d>, 20>,
}

impl WifiManager<'static> {
    pub fn init(
        wifi_init: &'static EspWifiController<'static>,
        wifi: peripherals::WIFI<'static>,
        bt: peripherals::BT<'static>,
    ) -> Self {
        //
        let (wifi_controller, interfaces) =
            esp_wifi::wifi::new(wifi_init, wifi).expect("Failed to initialize WIFI controller");

        // let transport = BleConnector::new(wifi_init, bt);
        // let ble_controller = ExternalController::<_, 20>::new(transport);

        debug!("wifi/ble initialized");

        Self {
            interfaces,
            wifi_controller,
            // ble_controller,
        }
    }

    pub async fn connect_to_hotspot<'d>(self, mut rng: Rng, spawner: Spawner) -> Stack<'d> {
        let dhcp_config = embassy_net::Config::dhcpv4(Default::default());
        let seed = (rng.random() as u64) << 32 | rng.random() as u64;

        let (stack, runner) = embassy_net::new(
            self.interfaces.sta,
            dhcp_config,
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed,
        );

        spawner.spawn(connection(self.wifi_controller)).ok();
        spawner.spawn(net_task(runner)).ok();

        loop {
            if stack.is_link_up() {
                info!("Link is up!");
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }

        info!("Waiting for ip...");
        loop {
            if let Some(config) = stack.config_v4() {
                info!("Got IP: {}", config.address);
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }

        return stack;
    }
}

pub struct HttpClient<'a> {
    socket: TcpSocket<'a>,
}

impl<'a> HttpClient<'a> {
    pub async fn connect(
        stack: Stack<'a>,
        remote: (embassy_net::Ipv4Address, u16),
    ) -> Result<Self, embassy_net::tcp::ConnectError> {
        static mut RX: [u8; 4096] = [0; 4096];
        static mut TX: [u8; 4096] = [0; 4096];

        let rx_buffer = unsafe { &mut RX };
        let tx_buffer = unsafe { &mut TX };

        let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);

        socket.set_timeout(Some(Duration::from_secs(10)));
        socket.connect(remote).await?;

        Ok(Self { socket })
    }

    pub async fn request(&mut self, req: &str) -> Result<(), embassy_net::tcp::Error> {
        use embedded_io_async::Write;
        self.socket.write_all(req.as_bytes()).await?;

        let mut buf = [0; 1024];

        let mut n = 0;
        loop {
            n += match self.socket.read(&mut buf).await {
                Ok(0) => {
                    info!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    error!("failed to read response: {:?}", e);
                    break;
                }
            }
        }

        info!("recieved {} bytes:", n);
        info!("Response: {}", core::str::from_utf8(&buf[..n]).unwrap());

        Ok(())
    }
}

pub struct SessionResultClient<'a> {
    http_client: HttpClient<'a>,
}

impl<'a> SessionResultClient<'a> {
    pub async fn new(stack: Stack<'a>) -> Result<Self, embassy_net::tcp::ConnectError> {
        let http_client = HttpClient::connect(stack, REMOTE_ENDPOINT).await?;

        Ok(Self { http_client })
    }

    pub async fn publish_result(
        &mut self,
        result: SessionResult,
    ) -> Result<(), embassy_net::tcp::Error> {
        let body = format!(
            "\
            {{\
                \"rate\": {},\
                \"duration\": {},\
                \"volume\": {}\
            }}\
            ",
            result.rate,
            result.duration.as_millis(),
            result.volume,
        );

        let request = format!(
            "\
            POST {} HTTP/1.1\r\n\
            Host: {} \r\n\
            Authorization: Basic {}\r\n\
            Content-Type: application/json\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}\
            ",
            RESULT_RESOURCE,
            REMOTE_ENDPOINT_STR,
            REMOTE_DEV_SECRET,
            body.len(),
            body
        );

        info!("Would send request: {:?}", request);
        self.http_client.request(request.as_str()).await?;

        Ok(())
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("start connection task");
    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started!");

            info!("Scan");
            let result = controller.scan_n_async(10).await.unwrap();
            for ap in result {
                info!("{:?}", ap);
            }
        }
        info!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => info!("Wifi connected!"),
            Err(e) => {
                info!("Failed to connect to wifi: {:#?}", e);
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
