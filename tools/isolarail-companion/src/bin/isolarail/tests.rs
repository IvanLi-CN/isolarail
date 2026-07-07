#[cfg(test)]
mod power_output_tests {
    use super::*;

    #[test]
    fn ensure_success_envelope_rejects_jsonl_ok_false() {
        let value = json!({
            "ok": false,
            "error": {"message": "port is locked"}
        });
        let err = ensure_success_envelope(&value).expect_err("ok=false should fail");
        assert!(err.to_string().contains("port is locked"));
    }

    #[test]
    fn ensure_success_envelope_ignores_non_envelope_output() {
        ensure_success_envelope(&json!({"devices": []})).expect("list output should pass");
        ensure_success_envelope(&json!({"ok": true})).expect("ok=true should pass");
    }

    #[test]
    fn human_output_renders_hardware_available_sections() {
        let output = json!({
            "path": "/tmp/devices.json",
            "saved": [{
                "id": "isolarail-01",
                "name": "Bench Hub",
                "transport": {
                    "kind": "usb",
                    "deviceId": "usb--dev-cu-usbmodem101"
                }
            }],
            "devd": {
                "devices": [{
                    "id": "usb--dev-cu-usbmodem101",
                    "displayName": "ESP32-S3 USB JTAG",
                    "connection": "available"
                }]
            }
        });

        let rendered = format_human_output(&output);
        assert!(rendered.contains("Registry: /tmp/devices.json"));
        assert!(rendered.contains("Saved hardware:"));
        assert!(rendered.contains("- Bench Hub (isolarail-01) usb:usb--dev-cu-usbmodem101"));
        assert!(rendered.contains("Local devd devices:"));
        assert!(rendered.contains("- ESP32-S3 USB JTAG (usb--dev-cu-usbmodem101) - available"));
    }

    #[test]
    fn human_output_renders_envelope_result_instead_of_plain_ok() {
        let output = json!({
            "ok": true,
            "result": {
                "device": {
                    "device_id": "f1fb44",
                    "hostname": "isolarail-f1fb44"
                }
            }
        });

        let rendered = format_human_output(&output);
        assert!(rendered.contains("\"device_id\": \"f1fb44\""));
        assert!(rendered.contains("\"hostname\": \"isolarail-f1fb44\""));
        assert!(!rendered.trim().eq("ok"));
    }

    #[test]
    fn maps_http_port_mutation_endpoints() {
        let (_, path, body) =
            map_http_endpoint(Method::POST, "/ports/port1/power?enabled=false", None)
                .expect("power endpoint should map");
        assert_eq!(path, "/api/v1/ports/port1/power?enabled=false");
        assert!(body.is_none());

        let (_, path, _) = map_http_endpoint(Method::POST, "/ports/port4/replug", None)
            .expect("replug endpoint should map");
        assert_eq!(path, "/api/v1/ports/port4/actions/replug");
    }

    #[test]
    fn maps_devd_device_endpoints_to_ipc_methods() {
        let (method, params) = map_devd_ipc_endpoint(
            Method::POST,
            "/api/v1/devices/usb--dev-cu-usbmodem21221401/ports/port1/power?enabled=false",
            None,
        )
        .expect("power endpoint should map");
        assert_eq!(method, "device.port.power");
        assert_eq!(params["device_id"], "usb--dev-cu-usbmodem21221401");
        assert_eq!(params["port"], "port1");
        assert_eq!(params["enabled"], false);
    }

    #[test]
    fn maps_device_diagnostics_endpoint_to_ipc_method() {
        let (method, params) = map_devd_ipc_endpoint(
            Method::GET,
            "/api/v1/devices/usb--dev-cu-usbmodem21221401/diagnostics",
            None,
        )
        .expect("diagnostics endpoint should map");
        assert_eq!(method, "device.diagnostics");
        assert_eq!(params["device_id"], "usb--dev-cu-usbmodem21221401");
    }

    #[test]
    fn maps_diag_snapshot_endpoint_to_ipc_method() {
        let (method, params) = map_devd_ipc_endpoint(
            Method::GET,
            "/api/v1/devices/usb--dev-cu-usbmodem21221401/diag-snapshot",
            None,
        )
        .expect("diag snapshot endpoint should map");
        assert_eq!(method, "device.hardware.snapshot");
        assert_eq!(params["device_id"], "usb--dev-cu-usbmodem21221401");
    }

    #[test]
    fn cli_uses_ipc_instead_of_devd_http_flag() {
        let cli = Cli::try_parse_from([
            "isolarail",
            "--ipc",
            "/tmp/isolarail-test.sock",
            "--no-auto-start",
            "devices",
        ])
        .expect("ipc flags should parse");
        assert_eq!(cli.ipc, "/tmp/isolarail-test.sock");
        assert!(cli.no_auto_start);

        let err = Cli::try_parse_from(["isolarail", "--devd", "http://127.0.0.1:51200", "devices"])
            .expect_err("legacy devd HTTP flag must not parse");
        assert!(err.to_string().contains("unexpected argument"));
    }

    #[test]
    fn devd_start_gate_allows_only_one_spawner_per_endpoint() {
        let endpoint = format!(
            "{}/isolarail-test-{}.sock",
            std::env::temp_dir().display(),
            std::process::id()
        );
        clear_devd_start_gate(&endpoint).expect("cleanup before test");

        let first = acquire_devd_start_gate(&endpoint).expect("first gate");
        assert!(matches!(first, DevdStartMode::Spawned { .. }));

        let second = acquire_devd_start_gate(&endpoint).expect("second gate");
        assert!(matches!(second, DevdStartMode::WaitingForExisting));

        drop(first);

        let third = acquire_devd_start_gate(&endpoint).expect("third gate");
        assert!(matches!(third, DevdStartMode::Spawned { .. }));
        drop(third);
        clear_devd_start_gate(&endpoint).expect("cleanup after test");
    }

    #[test]
    fn transient_ipc_errors_include_empty_response_after_connect() {
        let connect_err = anyhow::anyhow!("connect IPC socket /tmp/isolarail.sock");
        assert!(looks_like_transient_ipc_error(&connect_err));

        let empty_response_err =
            anyhow::anyhow!("IPC daemon closed the connection without a response");
        assert!(looks_like_transient_ipc_error(&empty_response_err));

        let refused_err = anyhow::anyhow!("Connection refused (os error 61)")
            .context("connect IPC socket /tmp/isolarail.sock");
        assert!(looks_like_transient_ipc_error(&refused_err));

        let request_err = anyhow::anyhow!("device busy: another Local USB operation is still running");
        assert!(!looks_like_transient_ipc_error(&request_err));
    }

    #[test]
    fn ports_power_accepts_explicit_boolean_value() {
        let cli = Cli::try_parse_from([
            "isolarail",
            "ports",
            "--device",
            "usb--dev-cu-usbmodem21221401",
            "power",
            "--port",
            "port1",
            "--enabled",
            "false",
        ])
        .expect("explicit boolean value should parse");

        let Command::Ports {
            command: Some(PortsCommand::Power { enabled, .. }),
            ..
        } = cli.command
        else {
            panic!("expected ports power command");
        };
        assert!(!enabled);
    }

    #[test]
    fn flash_accepts_non_project_confirmation_flag() {
        let cli = Cli::try_parse_from([
            "isolarail",
            "flash",
            "--device",
            "usb--dev-cu-usbmodem21221401",
            "--catalog",
            "catalog.json",
            "--artifact",
            "app",
            "--real",
            "--first-time",
            "--confirm-non-project-firmware",
        ])
        .expect("confirmation flag should parse");

        let Command::Flash(args) = cli.command else {
            panic!("expected flash command");
        };
        assert!(args.confirm_non_project_firmware);
    }

    #[test]
    fn wifi_set_selector_rejects_url_path() {
        let err = resolve_usb_capable_selector(
            ApiSelectorArgs {
                hardware: None,
                device: None,
                url: Some("http://isolarail-856a14.local".to_string()),
            },
            "/tmp/isolarail.sock",
            "Wi-Fi configuration changes",
        )
        .expect_err("url selector must be rejected for Wi-Fi writes");

        assert!(err.to_string().contains("Local USB in the CLI"));
        assert!(err.to_string().contains("--url is read-only"));
    }

    #[test]
    fn wifi_set_selector_accepts_device_path() {
        let selected = resolve_usb_capable_selector(
            ApiSelectorArgs {
                hardware: None,
                device: Some("usb--dev-cu-usbmodem2123101".to_string()),
                url: None,
            },
            "/tmp/isolarail.sock",
            "Wi-Fi configuration changes",
        )
        .expect("device selector should be accepted");

        assert_eq!(selected.device, "usb--dev-cu-usbmodem2123101");
        assert_eq!(selected.devd, "/tmp/isolarail.sock");
    }

    #[test]
    fn wifi_set_selector_rejects_http_saved_hardware() {
        with_temp_hardware_registry(
            vec![DeviceProfile {
                id: "bench-http".to_string(),
                name: "Bench Hub Wi-Fi".to_string(),
                transport: HardwareTransport::Http {
                    base_url: "http://isolarail-856a14.local".to_string(),
                },
                identity: Some(DeviceIdentity {
                    device_id: Some("856a14".to_string()),
                    mac: Some("AA:BB:CC:85:6A:14".to_string()),
                }),
                last_seen_at: Some(1),
            }],
            || {
                let err = resolve_usb_capable_selector(
                    ApiSelectorArgs {
                        hardware: Some("bench-http".to_string()),
                        device: None,
                        url: None,
                    },
                    "/tmp/isolarail.sock",
                    "Wi-Fi configuration changes",
                )
                .expect_err("HTTP saved hardware must be rejected for Wi-Fi writes");

                assert!(err.to_string().contains("saved hardware bench-http"));
                assert!(err.to_string().contains("Wi-Fi/LAN only"));
            },
        );
    }

    #[test]
    fn wifi_set_selector_accepts_usb_saved_hardware() {
        with_temp_hardware_registry(
            vec![DeviceProfile {
                id: "bench-usb".to_string(),
                name: "Bench Hub".to_string(),
                transport: HardwareTransport::Usb {
                    device_id: "usb--dev-cu-usbmodem2123101".to_string(),
                    devd_url: None,
                },
                identity: Some(DeviceIdentity {
                    device_id: Some("856a14".to_string()),
                    mac: Some("AA:BB:CC:85:6A:14".to_string()),
                }),
                last_seen_at: Some(1),
            }],
            || {
                let selected = resolve_usb_capable_selector(
                    ApiSelectorArgs {
                        hardware: Some("bench-usb".to_string()),
                        device: None,
                        url: None,
                    },
                    "/tmp/isolarail.sock",
                    "Wi-Fi configuration changes",
                )
                .expect("USB saved hardware should be accepted");

                assert_eq!(selected.device, "usb--dev-cu-usbmodem2123101");
                assert_eq!(
                    selected
                        .identity
                        .as_ref()
                        .and_then(|identity| identity.device_id.as_deref()),
                    Some("856a14")
                );
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceIdentity, DeviceProfile, DiscoverFirmware, HardwareTransport,
        discover_usb_scan_method, discover_usb_scan_path,
        parse_discovered_http_info, saved_hardware_match_for_transport,
    };
    use reqwest::Method;
    use serde_json::json;

    #[test]
    fn discover_scan_flag_maps_to_expected_devd_endpoint() {
        assert_eq!(discover_usb_scan_method(false), Method::GET);
        assert_eq!(discover_usb_scan_path(false), "/api/v1/devices");
        assert_eq!(discover_usb_scan_method(true), Method::POST);
        assert_eq!(discover_usb_scan_path(true), "/api/v1/devices/scan");
    }

    #[test]
    fn parse_discover_http_info_prefers_fqdn_base_url() {
        let parsed = parse_discovered_http_info(
            "http://192.168.1.42",
            json!({
                "device": {
                    "device_id": "aabbccdd",
                    "hostname": "isolarail-aabbcc",
                    "fqdn": "isolarail-aabbcc.local",
                    "mac": "AA:BB:CC:DD:EE:FF",
                    "firmware": {
                        "name": "isolarail",
                        "version": "0.1.0"
                    },
                    "wifi": {
                        "ipv4": "192.168.1.42"
                    }
                }
            }),
            Some(std::net::Ipv4Addr::new(192, 168, 1, 42)),
        )
        .expect("discover info should parse");

        assert_eq!(parsed.base_url, "http://isolarail-aabbcc.local");
        assert_eq!(parsed.ipv4.as_deref(), Some("192.168.1.42"));
        let identity = parsed.identity.expect("identity should exist");
        assert_eq!(identity.device_id.as_deref(), Some("aabbccdd"));
        assert_eq!(identity.mac.as_deref(), Some("AA:BB:CC:DD:EE:FF"));
        assert_eq!(
            parsed.firmware,
            DiscoverFirmware {
                name: "isolarail".to_string(),
                version: "0.1.0".to_string(),
            }
        );
    }

    #[test]
    fn saved_hardware_match_uses_canonical_owner_facing_name() {
        let saved = vec![
            DeviceProfile {
                id: "isolarail-01".to_string(),
                name: "Bench Hub".to_string(),
                transport: HardwareTransport::Usb {
                    device_id: "usb--dev-cu-usbmodem21221401".to_string(),
                    devd_url: None,
                },
                identity: Some(DeviceIdentity {
                    device_id: Some("856a14".to_string()),
                    mac: Some("AA:BB:CC:85:6A:14".to_string()),
                }),
                last_seen_at: None,
            },
            DeviceProfile {
                id: "isolarail-01-wifi".to_string(),
                name: "Bench Hub Wi-Fi".to_string(),
                transport: HardwareTransport::Http {
                    base_url: "http://isolarail-856a14.local".to_string(),
                },
                identity: Some(DeviceIdentity {
                    device_id: Some("856a14".to_string()),
                    mac: Some("AA:BB:CC:85:6A:14".to_string()),
                }),
                last_seen_at: None,
            },
        ];

        let usb_match = saved_hardware_match_for_transport(
            &saved,
            &[
                "usb:usb--dev-cu-usbmodem21221401".to_string(),
                "device:856a14".to_string(),
            ],
            Some("usb"),
        );

        let http_match = saved_hardware_match_for_transport(
            &saved,
            &[
                "http:http://isolarail-856a14.local".to_string(),
                "device:856a14".to_string(),
            ],
            Some("http"),
        );

        assert_eq!(usb_match.len(), 1);
        assert_eq!(usb_match[0].id, "isolarail-01");
        assert_eq!(usb_match[0].name, "Bench Hub");
        assert_eq!(http_match.len(), 1);
        assert_eq!(http_match[0].id, "isolarail-01");
        assert_eq!(http_match[0].name, "Bench Hub");
        assert_eq!(http_match[0].transport, "http");
    }
}

#[cfg(test)]
fn with_temp_hardware_registry<T>(devices: Vec<DeviceProfile>, run: impl FnOnce() -> T) -> T {
    use std::sync::{Mutex, OnceLock};

    static TEST_REGISTRY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    let _guard = TEST_REGISTRY_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test registry lock");
    let temp = tempfile::tempdir().expect("temp dir");
    let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
    let previous_home = std::env::var_os("HOME");

    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        std::env::set_var("HOME", temp.path());
    }

    isolarail_companion::write_hardware_registry(&isolarail_companion::HardwareRegistry {
        schema_version: 1,
        devices,
    })
    .expect("write registry");

    let result = run();

    unsafe {
        match previous_xdg {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }

    result
}
