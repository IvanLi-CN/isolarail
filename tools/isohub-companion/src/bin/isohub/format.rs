fn print_human(output: &Value) {
    print!("{}", format_human_output(output));
}

fn format_human_output(output: &Value) -> String {
    if output.get("saved").is_some() || output.get("devd").is_some() {
        return format_hardware_available(output);
    }

    if output.get("logs").is_some() && output.get("traces").is_some() {
        return format_session_output(output);
    }

    if let Some(devices) = output.get("devices").and_then(Value::as_array)
        && devices
            .first()
            .is_some_and(|device| device.get("transport").is_some())
    {
        return format_discover_output(output);
    }

    if let Some(devices) = output.get("devices").and_then(Value::as_array) {
        if devices.is_empty() {
            return "No devices found.\n".to_string();
        }
        let mut lines = Vec::new();
        for device in devices {
            let id = device
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown-device");
            let name = device
                .get("displayName")
                .or_else(|| device.get("display_name"))
                .or_else(|| device.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(id);
            let connection = device
                .get("connection")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            lines.push(format!("{name} ({id}) - {connection}"));
        }
        return format!("{}\n", lines.join("\n"));
    }

    if let Some(path) = output.get("path").and_then(Value::as_str) {
        return format!("{path}\n");
    }

    if let Some(result) = output.get("result") {
        return format_human_output(result);
    }

    if let Some(ok) = output.get("ok").and_then(Value::as_bool) {
        return format!("{}\n", if ok { "ok" } else { "failed" });
    }

    format!(
        "{}\n",
        serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
    )
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CliSessionItem {
    id: String,
    timestamp_unix_ms: u128,
    level: String,
    message: String,
    payload: Value,
}

#[derive(Debug, Clone)]
struct CliSessionRow {
    channel: &'static str,
    item: CliSessionItem,
}

fn format_session_output(output: &Value) -> String {
    let rows = collect_session_rows(output, None);
    if rows.is_empty() {
        return "No serial activity recorded yet.\n".to_string();
    }

    let mut lines = Vec::new();
    lines.push("Serial activity".to_string());
    for row in rows {
        lines.push(render_session_row(&row));
    }
    format!("{}\n", lines.join("\n"))
}

fn collect_session_rows(
    output: &Value,
    seen: Option<&mut std::collections::HashSet<String>>,
) -> Vec<CliSessionRow> {
    let mut rows = Vec::new();
    let mut seen = seen;
    for (channel, key) in [("log", "logs"), ("trace", "traces")] {
        let Some(items) = output.get(key).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let Ok(item) = serde_json::from_value::<CliSessionItem>(item.clone()) else {
                continue;
            };
            if let Some(seen) = seen.as_deref_mut() && !seen.insert(item.id.clone()) {
                continue;
            }
            rows.push(CliSessionRow { channel, item });
        }
    }
    rows.sort_by_key(|row| row.item.timestamp_unix_ms);
    rows
}

fn render_session_row(row: &CliSessionRow) -> String {
    let detail = render_session_detail(&row.item.payload);
    if detail.is_empty() {
        return format!(
            "[{}/{}] {}",
            row.channel, row.item.level, row.item.message
        );
    }
    format!(
        "[{}/{}] {} :: {}",
        row.channel, row.item.level, row.item.message, detail
    )
}

fn render_session_detail(payload: &Value) -> String {
    if let Some(detail) = payload.get("payload").and_then(Value::as_str) {
        return detail.to_string();
    }
    compact_json(payload)
}

fn compact_json(value: &Value) -> String {
    let mut text = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    if text.len() > 220 {
        text.truncate(217);
        text.push_str("...");
    }
    text
}

fn format_discover_output(output: &Value) -> String {
    let mut lines = Vec::new();
    if let Some(warnings) = output.get("warnings").and_then(Value::as_array) {
        for warning in warnings.iter().filter_map(Value::as_str) {
            lines.push(format!("warning: {warning}"));
        }
        if !warnings.is_empty() {
            lines.push(String::new());
        }
    }

    let Some(devices) = output.get("devices").and_then(Value::as_array) else {
        return "No devices found.\n".to_string();
    };
    if devices.is_empty() {
        if lines.is_empty() {
            return "No devices found.\n".to_string();
        }
        lines.push("No devices found.".to_string());
        return format!("{}\n", lines.join("\n"));
    }

    for device in devices {
        let transport = device.get("transport").unwrap_or(&Value::Null);
        let kind = transport
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let display_name = device
            .get("displayName")
            .or_else(|| device.get("display_name"))
            .and_then(Value::as_str)
            .unwrap_or("unknown-device");
        let mut detail = match kind {
            "http" => device
                .get("deviceId")
                .or_else(|| device.get("device_id"))
                .and_then(Value::as_str)
                .map(|device_id| format!("LAN {display_name} ({device_id})"))
                .unwrap_or_else(|| format!("LAN {display_name}")),
            "usb" => transport
                .get("deviceId")
                .or_else(|| transport.get("device_id"))
                .and_then(Value::as_str)
                .map(|device_id| format!("USB {display_name} ({device_id})"))
                .unwrap_or_else(|| format!("USB {display_name}")),
            _ => display_name.to_string(),
        };

        let endpoint = match kind {
            "http" => transport
                .get("baseUrl")
                .or_else(|| transport.get("base_url"))
                .and_then(Value::as_str),
            "usb" => transport
                .get("portPath")
                .or_else(|| transport.get("port_path"))
                .and_then(Value::as_str),
            _ => None,
        };
        if let Some(endpoint) = endpoint {
            detail.push_str(" - ");
            detail.push_str(endpoint);
        }

        let saved = device
            .get("savedHardware")
            .or_else(|| device.get("saved_hardware"))
            .and_then(Value::as_array)
            .map(|saved| {
                saved
                    .iter()
                    .filter_map(|entry| {
                        let id = entry.get("id").and_then(Value::as_str)?;
                        let name = entry.get("name").and_then(Value::as_str).unwrap_or(id);
                        Some(format!("{name} ({id})"))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !saved.is_empty() {
            detail.push_str(" [saved: ");
            detail.push_str(&saved.join(", "));
            detail.push(']');
        }

        lines.push(detail);
    }

    format!("{}\n", lines.join("\n"))
}

fn format_hardware_available(output: &Value) -> String {
    let mut lines = Vec::new();
    if let Some(path) = output.get("path").and_then(Value::as_str) {
        lines.push(format!("Registry: {path}"));
    }

    lines.push("Saved hardware:".to_string());
    match output.get("saved").and_then(Value::as_array) {
        Some(saved) if !saved.is_empty() => {
            for device in saved {
                let id = device
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown-hardware");
                let name = device.get("name").and_then(Value::as_str).unwrap_or(id);
                lines.push(format!("- {name} ({id}) {}", transport_label(device)));
            }
        }
        _ => lines.push("- none".to_string()),
    }

    lines.push("Local devd devices:".to_string());
    if let Some(error) = output
        .get("devd")
        .and_then(|devd| devd.get("error"))
        .and_then(Value::as_str)
    {
        lines.push(format!("- unavailable: {error}"));
    } else {
        match output
            .get("devd")
            .and_then(|devd| devd.get("devices"))
            .and_then(Value::as_array)
        {
            Some(devices) if !devices.is_empty() => {
                for device in devices {
                    let id = device
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown-device");
                    let name = device
                        .get("displayName")
                        .or_else(|| device.get("display_name"))
                        .or_else(|| device.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or(id);
                    let connection = device
                        .get("connection")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    lines.push(format!("- {name} ({id}) - {connection}"));
                }
            }
            _ => lines.push("- none".to_string()),
        }
    }

    format!("{}\n", lines.join("\n"))
}

fn transport_label(device: &Value) -> String {
    let Some(transport) = device.get("transport") else {
        return "(unknown transport)".to_string();
    };
    let kind = transport
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    match kind {
        "usb" => transport
            .get("deviceId")
            .or_else(|| transport.get("device_id"))
            .and_then(Value::as_str)
            .map(|device_id| format!("usb:{device_id}"))
            .unwrap_or_else(|| "usb".to_string()),
        "http" => transport
            .get("baseUrl")
            .or_else(|| transport.get("base_url"))
            .and_then(Value::as_str)
            .map(|base_url| format!("http:{base_url}"))
            .unwrap_or_else(|| "http".to_string()),
        "webSerial" | "web_serial" => "web_serial".to_string(),
        other => other.to_string(),
    }
}
