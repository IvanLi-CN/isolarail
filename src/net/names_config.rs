fn wifi_state_str(state: WifiConnectionState) -> &'static str {
    match state {
        WifiConnectionState::Idle => "idle",
        WifiConnectionState::Connecting => "connecting",
        WifiConnectionState::Connected => "connected",
        WifiConnectionState::Error => "error",
    }
}

fn format_ipv4(ip: Ipv4Address) -> HString<16> {
    let o = ip.octets();
    let mut out: HString<16> = HString::new();
    let _ = core::write!(out, "{}.{}.{}.{}", o[0], o[1], o[2], o[3]);
    out
}

fn format_mac_lower(mac: [u8; 6]) -> HString<17> {
    let mut out: HString<17> = HString::new();
    let _ = core::write!(
        out,
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5]
    );
    out
}

fn derive_device_names(mac: [u8; 6]) -> DeviceNames {
    let short_id = mdns::short_id_from_mac(mac);
    let hostname = WIFI_HOSTNAME
        .map(sanitize_hostname)
        .filter(|hostname| !hostname.is_empty())
        .unwrap_or_else(|| mdns::hostname_from_short_id(short_id.as_str()));
    let hostname_fqdn = mdns::fqdn_from_hostname(hostname.as_str());

    DeviceNames {
        mac,
        short_id,
        hostname,
        hostname_fqdn,
    }
}

fn sanitize_hostname(raw: &str) -> HString<32> {
    let mut out: HString<32> = HString::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            if out.push(ch.to_ascii_lowercase()).is_err() {
                break;
            }
        }
    }
    out
}

fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let mut parts = [0u8; 4];
    let mut idx = 0;
    for part in s.split('.') {
        if idx >= 4 {
            return None;
        }
        parts[idx] = part.parse::<u8>().ok()?;
        idx += 1;
    }
    if idx != 4 {
        return None;
    }
    Some(Ipv4Address::new(parts[0], parts[1], parts[2], parts[3]))
}

fn netmask_to_prefix(mask: Ipv4Address) -> Option<u8> {
    let value = u32::from_be_bytes(mask.octets());
    let prefix = value.count_ones() as u8;
    let reconstructed = if prefix == 0 {
        0
    } else {
        u32::MAX.checked_shl((32 - prefix as u32) as u32)?
    };
    if reconstructed == value {
        Some(prefix)
    } else {
        None
    }
}

fn build_net_config_from_env(credentials: Option<&WifiCredentials>) -> (NetConfig, bool) {
    if let Some(static_ipv4) = credentials.and_then(|credentials| credentials.static_ipv4()) {
        let address = Ipv4Address::new(
            static_ipv4.address[0],
            static_ipv4.address[1],
            static_ipv4.address[2],
            static_ipv4.address[3],
        );
        let netmask = Ipv4Address::new(
            static_ipv4.netmask[0],
            static_ipv4.netmask[1],
            static_ipv4.netmask[2],
            static_ipv4.netmask[3],
        );
        let gateway = Ipv4Address::new(
            static_ipv4.gateway[0],
            static_ipv4.gateway[1],
            static_ipv4.gateway[2],
            static_ipv4.gateway[3],
        );
        if let Some(prefix) = netmask_to_prefix(netmask) {
            let mut dns_servers: Vec<Ipv4Address, 3> = Vec::new();
            if let Some(dns) = static_ipv4.dns {
                let dns_ip = Ipv4Address::new(dns[0], dns[1], dns[2], dns[3]);
                let _ = dns_servers.push(dns_ip);
            }
            let static_cfg = StaticConfigV4 {
                address: Ipv4Cidr::new(address, prefix),
                gateway: Some(gateway),
                dns_servers,
            };
            info!(
                "Wi-Fi using EEPROM static IPv4: addr={} prefix={} gw={}",
                address, prefix, gateway
            );
            return (NetConfig::ipv4_static(static_cfg), true);
        }
    }

    let static_ip = WIFI_STATIC_IP;
    let netmask = WIFI_NETMASK;
    let gateway = WIFI_GATEWAY;

    if let (Some(ip_s), Some(mask_s), Some(gw_s)) = (static_ip, netmask, gateway) {
        if let (Some(ip), Some(mask), Some(gw)) =
            (parse_ipv4(ip_s), parse_ipv4(mask_s), parse_ipv4(gw_s))
        {
            if let Some(prefix) = netmask_to_prefix(mask) {
                let cidr = Ipv4Cidr::new(ip, prefix);
                let mut dns_servers: Vec<Ipv4Address, 3> = Vec::new();

                if let Some(dns_s) = WIFI_DNS {
                    if let Some(dns_ip) = parse_ipv4(dns_s) {
                        let _ = dns_servers.push(dns_ip);
                    }
                }

                let static_cfg = StaticConfigV4 {
                    address: cidr,
                    gateway: Some(gw),
                    dns_servers,
                };

                info!(
                    "Wi-Fi using static IPv4: addr={} prefix={} gw={}",
                    ip, prefix, gw
                );
                return (NetConfig::ipv4_static(static_cfg), true);
            }
        }
    }

    build_net_config_dhcp()
}

fn build_net_config_dhcp() -> (NetConfig, bool) {
    info!("Wi-Fi using DHCPv4 for IPv4 configuration");
    (NetConfig::dhcpv4(DhcpConfig::default()), false)
}
