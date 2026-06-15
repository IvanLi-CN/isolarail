fn parse_one_of(raw: &str, allowed: &[u16], label: &str) -> Result<u16, String> {
    let parsed = raw
        .parse::<u16>()
        .map_err(|_| format!("expected {label}"))?;
    if allowed.contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!("expected {label}"))
    }
}

fn parse_pps3_limit_ma(raw: &str) -> Result<u16, String> {
    parse_one_of(raw, &[3000, 5000], "3000 or 5000")
}

fn parse_type_c_broadcast_ma(raw: &str) -> Result<u16, String> {
    parse_one_of(raw, &[500, 1500], "500 or 1500")
}

fn parse_fixed_pd_voltages(raw: &str) -> Result<Vec<u16>, String> {
    if raw.trim().is_empty() || raw.trim() == "none" {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    for part in raw.split(',') {
        let mv = parse_one_of(
            part.trim(),
            &[9000, 12000, 15000, 20000],
            "9000,12000,15000,20000 or none",
        )?;
        if !values.contains(&mv) {
            values.push(mv);
        }
    }
    values.sort_unstable();
    Ok(values)
}

fn parse_scp_limit_ma(raw: &str) -> Result<u16, String> {
    parse_one_of(raw, &[2000, 4000, 5000], "2000, 4000, or 5000")
}

fn parse_fcp_afc_sfcp_limit_ma(raw: &str) -> Result<u16, String> {
    parse_one_of(raw, &[2250, 3250], "2250 or 3250")
}

fn next_power_owner() -> u32 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(1);
    let mixed = millis ^ ((std::process::id() as u64) << 16);
    let owner = (mixed as u32) & 0x7fff_ffff;
    owner.max(1)
}

fn set_protocol_flag(protocols: &mut Value, key: &str, enabled: bool) -> anyhow::Result<()> {
    let map = protocols
        .as_object_mut()
        .ok_or_else(|| anyhow!("power config protocols payload is not an object"))?;
    map.insert(key.to_string(), json!(enabled));
    Ok(())
}

fn apply_source_capability_args(
    config: &mut CliPowerConfig,
    args: &SourceCapabilitySetArgs,
) -> anyhow::Result<()> {
    if let Some(power_watts) = args.power_watts {
        config.capability.power_watts = power_watts;
    }
    if let Some(pd) = args.pd {
        set_protocol_flag(&mut config.capability.protocols, "pd", pd)?;
    }
    if let Some(pps) = args.pps {
        config.capability.pd.pps = pps;
    }
    if let Some(qc20) = args.qc20 {
        set_protocol_flag(&mut config.capability.protocols, "qc20", qc20)?;
    }
    if let Some(qc30) = args.qc30 {
        set_protocol_flag(&mut config.capability.protocols, "qc30", qc30)?;
    }
    if let Some(fcp) = args.fcp {
        set_protocol_flag(&mut config.capability.protocols, "fcp", fcp)?;
    }
    if let Some(afc) = args.afc {
        set_protocol_flag(&mut config.capability.protocols, "afc", afc)?;
    }
    if let Some(scp) = args.scp {
        set_protocol_flag(&mut config.capability.protocols, "scp", scp)?;
    }
    if let Some(pe20) = args.pe20 {
        set_protocol_flag(&mut config.capability.protocols, "pe20", pe20)?;
    }
    if let Some(bc12) = args.bc12 {
        set_protocol_flag(&mut config.capability.protocols, "bc12", bc12)?;
    }
    if let Some(sfcp) = args.sfcp {
        set_protocol_flag(&mut config.capability.protocols, "sfcp", sfcp)?;
    }
    if let Some(fixed_pd_voltages) = &args.fixed_pd_voltages {
        config.capability.pd.fixed_voltages_mv =
            parse_fixed_pd_voltages(fixed_pd_voltages).map_err(|err| anyhow!(err))?;
    }
    if let Some(pps3_limit_ma) = args.pps3_limit_ma {
        config.capability.current.pps3_limit_ma = pps3_limit_ma;
    }
    if let Some(pd_pps_5a) = args.pd_pps_5a {
        config.capability.current.pd_pps_5a = pd_pps_5a;
    }
    if let Some(type_c_broadcast_ma) = args.type_c_broadcast_ma {
        config.capability.current.type_c_broadcast_ma = type_c_broadcast_ma;
    }
    if let Some(scp_limit_ma) = args.scp_limit_ma {
        config.capability.current.scp_limit_ma = scp_limit_ma;
    }
    if let Some(fcp_afc_sfcp_limit_ma) = args.fcp_afc_sfcp_limit_ma {
        config.capability.current.fcp_afc_sfcp_limit_ma = fcp_afc_sfcp_limit_ma;
    }
    Ok(())
}

fn apply_manual_output_args(config: &mut CliPowerConfig, args: &ManualOutputArgs) {
    if let Some(voltage_mv) = args.voltage_mv {
        config.manual.voltage_mv = voltage_mv;
    }
    if let Some(current_limit_ma) = args.current_limit_ma {
        config.manual.current_limit_ma = current_limit_ma;
    }
    if let Some(usb_c_path) = args.usb_c_path {
        config.manual.usb_c_path_mode = usb_c_path.as_config_value().to_string();
    }
}

fn power_config_update_payload(config: &CliPowerConfig) -> Value {
    json!({
        "hardware": config.hardware,
        "tps_mode": config.tps_mode,
        "voltage_mv": config.manual.voltage_mv,
        "current_limit_ma": config.manual.current_limit_ma,
        "usb_c_path_mode": config.manual.usb_c_path_mode,
        "power_watts": config.capability.power_watts,
        "pd": protocol_enabled(&config.capability.protocols, "pd"),
        "qc20": protocol_enabled(&config.capability.protocols, "qc20"),
        "qc30": protocol_enabled(&config.capability.protocols, "qc30"),
        "fcp": protocol_enabled(&config.capability.protocols, "fcp"),
        "afc": protocol_enabled(&config.capability.protocols, "afc"),
        "scp": protocol_enabled(&config.capability.protocols, "scp"),
        "pe20": protocol_enabled(&config.capability.protocols, "pe20"),
        "bc12": protocol_enabled(&config.capability.protocols, "bc12"),
        "sfcp": protocol_enabled(&config.capability.protocols, "sfcp"),
        "pps": config.capability.pd.pps,
        "fixed_voltages_mv": config.capability.pd.fixed_voltages_mv,
        "pps3_limit_ma": config.capability.current.pps3_limit_ma,
        "pd_pps_5a": config.capability.current.pd_pps_5a,
        "type_c_broadcast_ma": config.capability.current.type_c_broadcast_ma,
        "scp_limit_ma": config.capability.current.scp_limit_ma,
        "fcp_afc_sfcp_limit_ma": config.capability.current.fcp_afc_sfcp_limit_ma,
    })
}

fn same_power_config_contents(left: &CliPowerConfig, right: &CliPowerConfig) -> bool {
    left.hardware == right.hardware
        && left.tps_mode == right.tps_mode
        && left.capability == right.capability
        && left.manual == right.manual
}

fn full_power_capability_defaults() -> CliPowerCapability {
    CliPowerCapability {
        profile: "full".to_string(),
        power_watts: 100,
        protocols: json!({
            "pd": true,
            "qc20": true,
            "qc30": true,
            "fcp": true,
            "afc": true,
            "scp": true,
            "pe20": true,
            "bc12": true,
            "sfcp": true,
        }),
        pd: CliPowerPd {
            pps: true,
            fixed_voltages_mv: vec![9000, 12000, 15000, 20000],
        },
        current: CliPowerCurrentProfile::default(),
    }
}

fn expected_default_power_config(current: &CliPowerConfig) -> CliPowerConfig {
    let mut expected = current.clone();
    expected.tps_mode = "auto_follow".to_string();
    expected.capability = full_power_capability_defaults();
    expected.manual = CliPowerManual {
        voltage_mv: MANUAL_OUTPUT_DEFAULT_VOLTAGE_MV,
        current_limit_ma: MANUAL_OUTPUT_DEFAULT_CURRENT_MA,
        usb_c_path_mode: "default".to_string(),
        path_policy: current
            .manual
            .path_policy
            .clone()
            .or_else(|| Some("auto".to_string())),
    };
    expected
}

async fn save_power_config_with_timeout_recovery(
    client: &Client,
    devd: &DevdClient,
    selector: &ApiSelectorArgs,
    owner: u32,
    config: &CliPowerConfig,
) -> anyhow::Result<Value> {
    let request = request_selected(
        client,
        devd,
        selector.clone(),
        Method::PUT,
        &format!("/power/config?owner={owner}"),
        Some(power_config_update_payload(config)),
    )
    .await;

    match request {
        Ok(value) => Ok(value),
        Err(err) if err.to_string().contains("serial response timed out") => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let observed = fetch_power_config(client, devd, selector).await?;
            if same_power_config_contents(&observed, config) {
                Ok(serde_json::to_value(observed)?)
            } else {
                Err(err)
            }
        }
        Err(err) => Err(err),
    }
}

async fn restore_power_defaults_with_timeout_recovery(
    client: &Client,
    devd: &DevdClient,
    selector: &ApiSelectorArgs,
    owner: u32,
) -> anyhow::Result<Value> {
    let expected =
        expected_default_power_config(&fetch_power_config(client, devd, selector).await?);
    let request = request_selected(
        client,
        devd,
        selector.clone(),
        Method::POST,
        &format!("/power/config/defaults?owner={owner}"),
        None,
    )
    .await;

    match request {
        Ok(value) => Ok(value),
        Err(err) if err.to_string().contains("serial response timed out") => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let observed = fetch_power_config(client, devd, selector).await?;
            if same_power_config_contents(&observed, &expected) {
                Ok(serde_json::to_value(observed)?)
            } else {
                Err(err)
            }
        }
        Err(err) => Err(err),
    }
}

fn protocol_enabled(protocols: &Value, key: &str) -> bool {
    protocols.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn toggle_fixed_pd_voltage(config: &mut CliPowerConfig, mv: u16) {
    if let Some(index) = config
        .capability
        .pd
        .fixed_voltages_mv
        .iter()
        .position(|value| *value == mv)
    {
        config.capability.pd.fixed_voltages_mv.remove(index);
    } else {
        config.capability.pd.fixed_voltages_mv.push(mv);
        config.capability.pd.fixed_voltages_mv.sort_unstable();
    }
}

const POWER_WATT_PRESETS: [u8; 6] = [15, 27, 45, 60, 65, 100];
const FIXED_PD_OPTIONS: [u16; 4] = [9000, 12000, 15000, 20000];
const ACTION_OPTIONS: [&str; 3] = ["Save and apply", "Reload from hardware", "Cancel"];
