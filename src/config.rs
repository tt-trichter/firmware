pub mod remote {
    use core::net::Ipv4Addr;
    pub const REMOTE_ENDPOINT: (Ipv4Addr, u16) = (Ipv4Addr::new(84, 164, 215, 229), 31080);
    // would ofc be better to derive from REMOTE_ENDPOINT
    pub const REMOTE_ENDPOINT_STR: &str = "84.164.215.229:31080";
    pub const RESULT_RESOURCE: &str = "/api/v1/results";
    pub const REMOTE_DEV_SECRET: &str = "dHJpY2h0ZXI6c3VwZXItc2FmZS1wYXNzd29yZA==";
}

pub mod wifi {
    pub const SSID: &str = env!("SSID");
    pub const PASSWORD: &str = env!("PASSWORD");
}
