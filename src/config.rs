pub mod remote {
    use core::net::Ipv4Addr;
    pub const REMOTE_ENDPOINT: (Ipv4Addr, u16) = (Ipv4Addr::new(4, 231, 40, 213), 80);
    // would ofc be better to derive from REMOTE_ENDPOINT
    pub const REMOTE_ENDPOINT_STR: &str = "4.231.40.213";
    pub const RESULT_RESOURCE: &str = "/api/v1/runs";
    pub const REMOTE_DEV_SECRET: &str = "dHJpY2h0ZXI6c3VwZXItc2FmZS1wYXNzd29yZA==";
}

pub mod wifi {
    pub const SSID: &str = "TrichterHotspot";
    pub const PASSWORD: &str = "Trichter12345678";
}

pub mod sensor {
    pub const STARTUP_DURATION_MS: u64 = 200;
    pub const STARTUP_REQUIRED_PULSES: u32 = 5;
}
