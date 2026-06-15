async fn fetch_power_config(
    client: &Client,
    devd: &DevdClient,
    selector: &ApiSelectorArgs,
) -> anyhow::Result<CliPowerConfig> {
    let current = request_selected(
        client,
        devd,
        selector.clone(),
        Method::GET,
        "/power/config",
        None,
    )
    .await?;
    Ok(serde_json::from_value(unwrap_device_success_result(
        current,
    )?)?)
}

async fn fetch_power_diagnostics(
    client: &Client,
    devd: &DevdClient,
    selector: &ApiSelectorArgs,
) -> anyhow::Result<CliPowerDiagnostics> {
    let current = request_selected(
        client,
        devd,
        selector.clone(),
        Method::GET,
        "/diagnostics",
        None,
    )
    .await?;
    Ok(serde_json::from_value(unwrap_device_success_result(
        current,
    )?)?)
}

fn saved_hardware_target_label(device: &DeviceProfile) -> String {
    let target = match &device.transport {
        HardwareTransport::Usb { device_id, .. } => format!("usb {device_id}"),
        HardwareTransport::Http { base_url } => format!("http {base_url}"),
        HardwareTransport::WebSerial { label } => label
            .as_ref()
            .map(|value| format!("web-serial {value}"))
            .unwrap_or_else(|| "web-serial".to_string()),
    };
    format!("{} ({}) - {}", device.name, device.id, target)
}

fn power_selector_to_api_selector(selector: PowerSelectorArgs) -> ApiSelectorArgs {
    ApiSelectorArgs {
        hardware: selector.hardware,
        device: None,
        url: None,
    }
}

#[derive(Clone)]
struct PowerTargetCandidate {
    hardware: DeviceProfile,
    verify_http_after_select: bool,
}

async fn finalize_power_target_candidate(
    client: &Client,
    devd: &DevdClient,
    candidate: PowerTargetCandidate,
) -> anyhow::Result<ApiSelectorArgs> {
    let selector = ApiSelectorArgs {
        hardware: Some(candidate.hardware.id.clone()),
        device: None,
        url: None,
    };
    if candidate.verify_http_after_select {
        request_selected(client, devd, selector.clone(), Method::GET, "/status", None)
            .await
            .with_context(|| {
                format!(
                    "saved LAN hardware {} is not reachable right now",
                    candidate.hardware.id
                )
            })?;
    }
    Ok(selector)
}

async fn collect_scanned_saved_usb_power_targets(
    client: &Client,
    devd: &DevdClient,
    saved: &[DeviceProfile],
) -> anyhow::Result<Vec<DeviceProfile>> {
    let scanned = devd_request(client, devd, Method::POST, "/api/v1/devices/scan", None).await?;
    let devices = scanned
        .get("devices")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("device scan returned no device list"))?
        .iter()
        .cloned()
        .map(serde_json::from_value::<DeviceRecord>)
        .collect::<Result<Vec<_>, _>>()?;

    let mut matched = Vec::new();
    for device in devices {
        let Some(_usb) = &device.usb else {
            continue;
        };
        if request_selected(
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
        .is_err()
        {
            continue;
        }
        if let Some(saved_device) = saved.iter().find(|saved_device| {
            matches!(
                &saved_device.transport,
                HardwareTransport::Usb { device_id, .. } if device_id == &device.id
            )
        }) {
            matched.push(saved_device.clone());
        }
    }
    Ok(matched)
}

async fn select_saved_power_target_interactively(
    client: &Client,
    devd: &DevdClient,
    selector: PowerSelectorArgs,
) -> anyhow::Result<ApiSelectorArgs> {
    if !selector.is_empty() {
        return Ok(power_selector_to_api_selector(selector));
    }
    if !io::stdin().is_terminal() {
        return Err(anyhow!(
            "select --hardware; interactive power target selection requires a terminal"
        ));
    }

    let mut saved = read_hardware_registry()?.devices;
    saved.retain(|device| !matches!(device.transport, HardwareTransport::WebSerial { .. }));
    saved.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));

    if saved.is_empty() {
        return Err(anyhow!(
            "no saved hardware is available; bind hardware first with `isohub hardware save`"
        ));
    }

    let lan_candidates = saved
        .iter()
        .filter(|device| matches!(device.transport, HardwareTransport::Http { .. }))
        .cloned()
        .map(|hardware| PowerTargetCandidate {
            hardware,
            verify_http_after_select: true,
        });
    let usb_candidates = collect_scanned_saved_usb_power_targets(client, devd, &saved)
        .await?
        .into_iter()
        .map(|hardware| PowerTargetCandidate {
            hardware,
            verify_http_after_select: false,
        });
    let mut candidates = lan_candidates.chain(usb_candidates).collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(anyhow!(
            "no power-control target is available; save a LAN target or connect a saved USB target so it appears in the current scan"
        ));
    }

    if candidates.len() == 1 {
        return finalize_power_target_candidate(client, devd, candidates.remove(0)).await;
    }

    let items = candidates
        .iter()
        .map(|candidate| saved_hardware_target_label(&candidate.hardware))
        .collect::<Vec<_>>();
    let selected = run_tui_list_menu(
        "Select saved hardware for power control",
        Some(
            "Saved LAN hardware is listed first. Saved USB hardware appears only after the current scan sees it online. Use Up/Down to move, Enter to select, Esc to cancel.",
        ),
        &items,
        &[],
    )?;
    let Some(selected) = selected else {
        return Err(UserCancelled.into());
    };
    finalize_power_target_candidate(client, devd, candidates.swap_remove(selected)).await
}

async fn select_api_target_interactively(
    client: &Client,
    devd: &DevdClient,
    selector: ApiSelectorArgs,
) -> anyhow::Result<ApiSelectorArgs> {
    if !selector.is_empty() {
        return Ok(selector);
    }
    if !io::stdin().is_terminal() {
        return Err(anyhow!(
            "select one of --hardware, --device, or --url; interactive device selection requires a terminal"
        ));
    }

    let scanned = devd_request(client, devd, Method::POST, "/api/v1/devices/scan", None).await?;
    let devices = scanned
        .get("devices")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("device scan returned no device list"))?
        .iter()
        .cloned()
        .map(serde_json::from_value::<DeviceRecord>)
        .collect::<Result<Vec<_>, _>>()?;

    if devices.is_empty() {
        return Err(anyhow!(
            "no devd devices found; connect hardware or pass --hardware/--device/--url explicitly"
        ));
    }

    let mut compatible = Vec::new();
    let mut rejected = Vec::new();
    for device in devices {
        let selector = ApiSelectorArgs {
            hardware: None,
            device: Some(device.id.clone()),
            url: None,
        };
        match request_selected(client, devd, selector.clone(), Method::GET, "/status", None).await {
            Ok(_) => compatible.push((device, selector)),
            Err(err) => rejected.push(format!("{} ({}) - {}", device.display_name, device.id, err)),
        }
    }

    if compatible.is_empty() {
        let mut message =
            String::from("no compatible IsoHub devices were found in the current scan");
        if !rejected.is_empty() {
            message.push_str(":\n");
            message.push_str(&rejected.join("\n"));
        }
        return Err(anyhow!(message));
    }

    if compatible.len() == 1 {
        return Ok(compatible.remove(0).1);
    }

    let items = compatible
        .iter()
        .map(|(device, _selector)| {
            let target = if let Some(usb) = &device.usb {
                format!("usb {}", usb.port_path)
            } else if let Some(http) = &device.http {
                format!("http {}", http.base_url)
            } else {
                device.connection.clone()
            };
            format!("{} ({}) - {}", device.display_name, device.id, target)
        })
        .collect::<Vec<_>>();
    let selected = run_tui_list_menu(
        "Select a device for source-capability editing",
        Some(
            "Only compatible IsoHub devices are shown. Use Up/Down to move, Enter to select, Esc to cancel.",
        ),
        &items,
        &[],
    )?;
    let Some(selected) = selected else {
        return Err(UserCancelled.into());
    };
    Ok(compatible.swap_remove(selected).1)
}

async fn run_source_capability_interactive(
    client: &Client,
    devd: &DevdClient,
    selector: ApiSelectorArgs,
) -> anyhow::Result<Value> {
    if !io::stdin().is_terminal() {
        return Err(anyhow!(
            "interactive source-capability editing requires a terminal; pass flags for non-interactive use"
        ));
    }

    let selector = select_api_target_interactively(client, devd, selector).await?;
    let mut config = fetch_power_config(client, devd, &selector).await?;
    let mut diagnostics = fetch_power_diagnostics(client, devd, &selector).await?;

    loop {
        let status = format_live_power_output(&serde_json::to_value(&diagnostics)?);
        match run_source_capability_editor_tui(&mut config, status.trim_end())? {
            EditorSubmit::Continue => continue,
            EditorSubmit::Save => {
                let owner = next_power_owner();
                return save_power_config_with_timeout_recovery(
                    client, devd, &selector, owner, &config,
                )
                .await;
            }
            EditorSubmit::Reload => {
                config = fetch_power_config(client, devd, &selector).await?;
                diagnostics = fetch_power_diagnostics(client, devd, &selector).await?;
            }
            EditorSubmit::Cancel => return Err(UserCancelled.into()),
        }
    }
}

async fn handle_power(
    client: &Client,
    devd: &DevdClient,
    command: PowerCommand,
    allow_interactive: bool,
) -> anyhow::Result<Value> {
    match command {
        PowerCommand::Show(selector) => {
            let selector =
                maybe_select_power_target(client, devd, selector, allow_interactive).await?;
            let config = unwrap_device_success_result(
                request_selected(
                    client,
                    devd,
                    selector.clone(),
                    Method::GET,
                    "/power/config",
                    None,
                )
                .await?,
            )?;
            let diagnostics = unwrap_device_success_result(
                request_selected(client, devd, selector, Method::GET, "/diagnostics", None).await?,
            )?;
            Ok(json!({
                "config": config,
                "diagnostics": diagnostics,
            }))
        }
        PowerCommand::Defaults { selector } => {
            let selector =
                maybe_select_power_target(client, devd, selector, allow_interactive).await?;
            let owner = next_power_owner();
            unwrap_device_success_result(
                restore_power_defaults_with_timeout_recovery(client, devd, &selector, owner)
                    .await?,
            )
        }
        PowerCommand::Output { command } => match command {
            OutputCommand::Manual { selector, args } => {
                let selector =
                    maybe_select_power_target(client, devd, selector, allow_interactive).await?;
                let owner = next_power_owner();
                let mut config = fetch_power_config(client, devd, &selector).await?;
                config.tps_mode = "manual".to_string();
                apply_manual_output_args(&mut config, &args);
                unwrap_device_success_result(
                    save_power_config_with_timeout_recovery(
                        client, devd, &selector, owner, &config,
                    )
                    .await?,
                )
            }
            OutputCommand::Auto { selector } => {
                let selector =
                    maybe_select_power_target(client, devd, selector, allow_interactive).await?;
                let owner = next_power_owner();
                let mut config = fetch_power_config(client, devd, &selector).await?;
                config.tps_mode = "auto_follow".to_string();
                unwrap_device_success_result(
                    save_power_config_with_timeout_recovery(
                        client, devd, &selector, owner, &config,
                    )
                    .await?,
                )
            }
        },
        PowerCommand::SourceCapability { command } => match command {
            SourceCapabilityCommand::Set { selector, args } => {
                let selector =
                    maybe_select_power_target(client, devd, selector, allow_interactive).await?;
                if !args.has_updates() {
                    if !allow_interactive {
                        return Err(anyhow!(
                            "interactive source-capability editing is unavailable with --json; pass one or more update flags"
                        ));
                    }
                    return run_source_capability_interactive(client, devd, selector).await;
                }
                let owner = next_power_owner();
                let mut config = fetch_power_config(client, devd, &selector).await?;
                apply_source_capability_args(&mut config, &args)?;
                unwrap_device_success_result(
                    save_power_config_with_timeout_recovery(
                        client, devd, &selector, owner, &config,
                    )
                    .await?,
                )
            }
        },
    }
}

async fn maybe_select_power_target(
    client: &Client,
    devd: &DevdClient,
    selector: PowerSelectorArgs,
    allow_interactive: bool,
) -> anyhow::Result<ApiSelectorArgs> {
    if allow_interactive {
        select_saved_power_target_interactively(client, devd, selector).await
    } else {
        Ok(power_selector_to_api_selector(selector))
    }
}
