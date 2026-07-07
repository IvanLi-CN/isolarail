#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiscoverDevice {
    id: String,
    display_name: String,
    connection: String,
    transport: DiscoverTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fqdn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ipv4: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    firmware: Option<DiscoverFirmware>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    saved_hardware: Option<Vec<DiscoverSavedHardware>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum DiscoverTransport {
    Usb {
        device_id: String,
        port_path: String,
    },
    Http {
        base_url: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiscoverFirmware {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DiscoverSavedHardware {
    id: String,
    name: String,
    transport: String,
}

#[derive(Debug, Deserialize)]
struct DiscoverApiInfoEnvelope {
    device: DiscoverApiInfoDevice,
}

#[derive(Debug, Deserialize)]
struct DiscoverApiInfoDevice {
    device_id: Option<String>,
    hostname: Option<String>,
    fqdn: Option<String>,
    mac: Option<String>,
    firmware: Option<DiscoverApiInfoFirmware>,
    wifi: Option<DiscoverApiInfoWifi>,
}

#[derive(Debug, Deserialize)]
struct DiscoverApiInfoFirmware {
    name: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscoverApiInfoWifi {
    ipv4: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedDiscoverHttpInfo {
    base_url: String,
    hostname: Option<String>,
    fqdn: Option<String>,
    ipv4: Option<String>,
    firmware: DiscoverFirmware,
    identity: Option<DeviceIdentity>,
}

async fn discover_usb_devices(
    client: &Client,
    devd: &DevdClient,
    scan: bool,
    saved: &[DeviceProfile],
) -> anyhow::Result<Vec<DiscoverDevice>> {
    let listed = devd_request(
        client,
        devd,
        discover_usb_scan_method(scan),
        discover_usb_scan_path(scan),
        None,
    )
    .await?;
    let devices = listed
        .get("devices")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("device scan returned no device list"))?
        .iter()
        .cloned()
        .map(serde_json::from_value::<DeviceRecord>)
        .collect::<Result<Vec<_>, _>>()?;

    let mut discovered = Vec::new();
    for device in devices {
        let Some(usb) = device.usb.clone() else {
            continue;
        };
        let info = request_selected(
            client,
            devd,
            ApiSelectorArgs {
                hardware: None,
                device: Some(device.id.clone()),
                url: None,
            },
            Method::GET,
            "/status",
            None,
        )
        .await
        .ok();
        let identity = info.as_ref().and_then(parse_device_identity_from_info);
        let firmware = info
            .as_ref()
            .and_then(parse_discovered_http_info_from_value)
            .map(|parsed| parsed.firmware);
        let mut keys = vec![format!("usb:{}", normalize_discovery_key(&device.id))];
        if let Some(identity) = &identity {
            extend_unique(&mut keys, discover_identity_match_keys(identity));
        }
        let saved_hardware = saved_hardware_match_for_transport(saved, &keys, Some("usb"));
        let display_name = identity
            .as_ref()
            .and_then(|identity| identity.device_id.clone())
            .unwrap_or_else(|| usb.label.clone());
        discovered.push(DiscoverDevice {
            id: device.id.clone(),
            display_name,
            connection: "available".to_string(),
            transport: DiscoverTransport::Usb {
                device_id: device.id,
                port_path: usb.port_path,
            },
            device_id: identity.and_then(|identity| identity.device_id),
            hostname: None,
            fqdn: None,
            ipv4: None,
            firmware,
            saved_hardware: (!saved_hardware.is_empty()).then_some(saved_hardware),
        });
    }
    Ok(discovered)
}

fn discover_usb_scan_method(scan: bool) -> Method {
    if scan {
        Method::POST
    } else {
        Method::GET
    }
}

fn discover_usb_scan_path(scan: bool) -> &'static str {
    if scan {
        "/api/v1/devices/scan"
    } else {
        "/api/v1/devices"
    }
}

async fn discover_lan_devices(
    client: &Client,
    saved: &[DeviceProfile],
) -> (Vec<DiscoverDevice>, Vec<String>) {
    let mdns = match ServiceDaemon::new() {
        Ok(mdns) => mdns,
        Err(err) => {
            return (
                Vec::new(),
                vec![format!("LAN discovery unavailable: {err}")],
            );
        }
    };
    let receiver = match mdns.browse("_http._tcp.local.") {
        Ok(receiver) => receiver,
        Err(err) => {
            return (
                Vec::new(),
                vec![format!("LAN discovery unavailable: {err}")],
            );
        }
    };

    let started = Instant::now();
    let mut devices = HashMap::<String, DiscoverDevice>::new();
    while started.elapsed() < Duration::from_secs(3) {
        let Ok(event) = receiver.recv_timeout(Duration::from_millis(200)) else {
            continue;
        };
        let ServiceEvent::ServiceResolved(service) = event else {
            continue;
        };
        if !service.is_valid() {
            continue;
        }

        let hostname = service.get_hostname().trim_end_matches('.').to_string();
        let port = service.get_port();
        let scanned_ipv4 = service.get_addresses_v4().into_iter().next();
        let validation_base_url = if let Some(ip) = scanned_ipv4 {
            if port == 80 {
                format!("http://{ip}")
            } else {
                format!("http://{ip}:{port}")
            }
        } else if port == 80 {
            format!("http://{hostname}")
        } else {
            format!("http://{hostname}:{port}")
        };

        let info = match fetch_http_info(client, &validation_base_url).await {
            Ok(info) => info,
            Err(_) => continue,
        };
        let Some(parsed) = parse_discovered_http_info(&validation_base_url, info, scanned_ipv4)
        else {
            continue;
        };
        let mut keys = vec![format!("http:{}", canonical_base_url(&parsed.base_url))];
        if let Some(identity) = &parsed.identity {
            extend_unique(&mut keys, discover_identity_match_keys(identity));
        }
        let saved_hardware = saved_hardware_match_for_transport(saved, &keys, Some("http"));
        let dedup_key = parsed
            .identity
            .as_ref()
            .and_then(|identity| identity.device_id.as_deref())
            .map(|device_id| format!("id:{}", normalize_discovery_key(device_id)))
            .unwrap_or_else(|| format!("url:{}", canonical_base_url(&parsed.base_url)));
        devices.insert(
            dedup_key,
            DiscoverDevice {
                id: parsed
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.device_id.clone())
                    .unwrap_or_else(|| parsed.base_url.clone()),
                display_name: parsed
                    .hostname
                    .clone()
                    .or_else(|| {
                        parsed
                            .identity
                            .as_ref()
                            .and_then(|identity| identity.device_id.clone())
                    })
                    .unwrap_or_else(|| parsed.base_url.clone()),
                connection: "available".to_string(),
                transport: DiscoverTransport::Http {
                    base_url: parsed.base_url,
                },
                device_id: parsed.identity.and_then(|identity| identity.device_id),
                hostname: parsed.hostname,
                fqdn: parsed.fqdn,
                ipv4: parsed.ipv4,
                firmware: Some(parsed.firmware),
                saved_hardware: (!saved_hardware.is_empty()).then_some(saved_hardware),
            },
        );
    }

    (devices.into_values().collect(), Vec::new())
}

async fn fetch_http_info(client: &Client, base_url: &str) -> anyhow::Result<Value> {
    Ok(client
        .get(api_url(base_url, "/api/v1/info")?)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?)
}

fn parse_discovered_http_info(
    base_url_by_ip: &str,
    value: Value,
    scanned_ipv4: Option<std::net::Ipv4Addr>,
) -> Option<ParsedDiscoverHttpInfo> {
    let env: DiscoverApiInfoEnvelope = serde_json::from_value(value).ok()?;
    let firmware_name = env.device.firmware.as_ref()?.name.as_deref()?.trim();
    if firmware_name != "isolarail" {
        return None;
    }
    let firmware = DiscoverFirmware {
        name: firmware_name.to_string(),
        version: env
            .device
            .firmware
            .as_ref()
            .and_then(|firmware| firmware.version.clone())
            .unwrap_or_else(|| "unknown".to_string()),
    };
    let fqdn = env.device.fqdn.and_then(non_empty_string);
    let base_url = if let Some(fqdn) = fqdn.as_deref().filter(|fqdn| fqdn.ends_with(".local")) {
        format!("http://{fqdn}")
    } else {
        base_url_by_ip.to_string()
    };
    let identity = match (
        env.device.device_id.and_then(non_empty_string),
        env.device.mac.and_then(non_empty_string),
    ) {
        (None, None) => None,
        (device_id, mac) => Some(DeviceIdentity { device_id, mac }),
    };
    Some(ParsedDiscoverHttpInfo {
        base_url,
        hostname: env.device.hostname.and_then(non_empty_string),
        fqdn,
        ipv4: env
            .device
            .wifi
            .and_then(|wifi| wifi.ipv4.and_then(non_empty_string))
            .or_else(|| scanned_ipv4.map(|ipv4| ipv4.to_string())),
        firmware,
        identity,
    })
}

fn parse_discovered_http_info_from_value(value: &Value) -> Option<ParsedDiscoverHttpInfo> {
    parse_discovered_http_info("http://127.0.0.1", value.clone(), None)
}

fn parse_device_identity_from_info(value: &Value) -> Option<DeviceIdentity> {
    let device = value.get("device")?;
    let device_id = device
        .get("device_id")
        .or_else(|| device.get("deviceId"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let mac = device
        .get("mac")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    if device_id.is_none() && mac.is_none() {
        None
    } else {
        Some(DeviceIdentity { device_id, mac })
    }
}

fn saved_hardware_match_for_transport(
    saved: &[DeviceProfile],
    live_keys: &[String],
    preferred_transport: Option<&str>,
) -> Vec<DiscoverSavedHardware> {
    let matches = saved
        .iter()
        .filter_map(|profile| {
            let score = saved_profile_match_score(profile, live_keys, preferred_transport)?;
            Some((score, profile))
        })
        .collect::<Vec<_>>();

    let Some((_, preferred_profile)) =
        matches
            .iter()
            .max_by(|(score_a, profile_a), (score_b, profile_b)| {
                score_a
                    .cmp(score_b)
                    .then_with(|| profile_a.last_seen_at.cmp(&profile_b.last_seen_at))
                    .then_with(|| profile_b.id.cmp(&profile_a.id))
            })
    else {
        return Vec::new();
    };

    let canonical_profile = matches
        .iter()
        .map(|(_, profile)| *profile)
        .max_by(|profile_a, profile_b| {
            saved_profile_canonical_rank(profile_a)
                .cmp(&saved_profile_canonical_rank(profile_b))
                .then_with(|| profile_a.last_seen_at.cmp(&profile_b.last_seen_at))
                .then_with(|| profile_b.id.cmp(&profile_a.id))
        })
        .unwrap_or(*preferred_profile);

    vec![DiscoverSavedHardware {
        id: canonical_profile.id.clone(),
        name: canonical_profile.name.clone(),
        transport: saved_profile_transport_name(preferred_profile).to_string(),
    }]
}

fn saved_profile_match_keys(profile: &DeviceProfile) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(identity) = &profile.identity {
        extend_unique(&mut keys, discover_identity_match_keys(identity));
    }
    match &profile.transport {
        HardwareTransport::Usb { device_id, .. } => {
            keys.push(format!("usb:{}", normalize_discovery_key(device_id)));
        }
        HardwareTransport::Http { base_url } => {
            keys.push(format!("http:{}", canonical_base_url(base_url)));
            if let Some(short_id) = default_hostname_short_id(base_url) {
                keys.push(format!("device:{short_id}"));
            }
        }
        HardwareTransport::WebSerial { .. } => {}
    }
    dedupe_strings(keys)
}

fn saved_profile_match_score(
    profile: &DeviceProfile,
    live_keys: &[String],
    preferred_transport: Option<&str>,
) -> Option<(bool, bool, usize)> {
    let profile_keys = saved_profile_match_keys(profile);
    let matched_keys = profile_keys
        .iter()
        .filter(|key| live_keys.iter().any(|live_key| live_key == *key))
        .cloned()
        .collect::<Vec<_>>();
    if matched_keys.is_empty() {
        return None;
    }

    let transport_name = saved_profile_transport_name(profile);
    let exact_transport_match = matched_keys
        .iter()
        .any(|key| key.starts_with(&format!("{transport_name}:")));
    let transport_preferred = preferred_transport == Some(transport_name);
    Some((
        exact_transport_match,
        transport_preferred,
        matched_keys.len(),
    ))
}

fn saved_profile_transport_name(profile: &DeviceProfile) -> &'static str {
    match &profile.transport {
        HardwareTransport::Usb { .. } => "usb",
        HardwareTransport::Http { .. } => "http",
        HardwareTransport::WebSerial { .. } => "web-serial",
    }
}

fn saved_profile_canonical_rank(profile: &DeviceProfile) -> (bool, bool, usize) {
    let has_identity = profile.identity.is_some();
    let transport_suffix_free = !looks_like_transport_qualified_name(&profile.name);
    let inverse_len = usize::MAX - profile.name.len();
    (has_identity, transport_suffix_free, inverse_len)
}

fn looks_like_transport_qualified_name(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    normalized.ends_with(" wifi")
        || normalized.ends_with("-wifi")
        || normalized.ends_with(" lan")
        || normalized.ends_with("-lan")
        || normalized.ends_with(" usb")
        || normalized.ends_with("-usb")
        || normalized.ends_with(" web serial")
        || normalized.ends_with("-web-serial")
}

fn discover_identity_match_keys(identity: &DeviceIdentity) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(device_id) = identity.device_id.as_deref().map(normalize_discovery_key) {
        keys.push(format!("device:{device_id}"));
    }
    if let Some(mac_short) = identity.mac.as_deref().and_then(mac_short_id) {
        keys.push(format!("device:{mac_short}"));
    }
    dedupe_strings(keys)
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();
    for value in values {
        if !deduped.iter().any(|existing| existing == &value) {
            deduped.push(value);
        }
    }
    deduped
}

fn extend_unique(target: &mut Vec<String>, extras: Vec<String>) {
    for extra in extras {
        if !target.iter().any(|existing| existing == &extra) {
            target.push(extra);
        }
    }
}

fn canonical_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/').to_string();
    reqwest::Url::parse(&trimmed)
        .ok()
        .map(|url| url.to_string().trim_end_matches('/').to_string())
        .unwrap_or(trimmed)
}

fn normalize_discovery_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn mac_short_id(value: &str) -> Option<String> {
    let hex = value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    if hex.len() < 6 {
        return None;
    }
    let short_id = &hex[hex.len() - 6..];
    (short_id.len() == 6 && short_id.chars().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| short_id.to_ascii_lowercase())
}

fn default_hostname_short_id(base_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(base_url).ok()?;
    let host = url.host_str()?.trim_end_matches('.').to_ascii_lowercase();
    let host = host.strip_suffix(".local").unwrap_or(&host);
    let short_id = host.strip_prefix("isolarail-")?;
    (short_id.len() == 6 && short_id.chars().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| short_id.to_string())
}
