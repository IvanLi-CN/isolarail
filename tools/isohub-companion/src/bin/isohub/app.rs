#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .context("build IsoHub companion HTTP client")?;
    let devd = DevdClient {
        endpoint: cli.ipc.clone(),
        auto_start: !cli.no_auto_start,
    };
    let value_result: anyhow::Result<Value> = async {
        Ok(match cli.command {
            Command::Discover { scan } => handle_discover(&client, &devd, scan).await?,
            Command::Devices => {
                devd_request(&client, &devd, Method::POST, "/api/v1/devices/scan", None).await?
            }
            Command::Status(selector) => {
                request_selected(&client, &devd, selector, Method::GET, "/status", None).await?
            }
            Command::DiagSnapshot(selector) => {
                let value =
                    request_selected(&client, &devd, selector, Method::GET, "/diag-snapshot", None)
                        .await?;
                value.get("result").cloned().unwrap_or(value)
            }
            Command::Hardware { command } => handle_hardware(&client, &devd, command).await?,
            Command::Wifi { command } => match command {
                WifiCommand::Show(selector) => {
                    request_selected(&client, &devd, selector, Method::GET, "/wifi", None).await?
                }
                WifiCommand::Set {
                    selector,
                    ssid,
                    psk,
                } => {
                    request_selected_usb_capable(
                        &client,
                        &devd,
                        selector,
                        Method::POST,
                        "/wifi",
                        Some(json!({"ssid": ssid, "psk": psk})),
                        "Wi-Fi configuration changes",
                    )
                    .await?
                }
                WifiCommand::Clear(selector) => {
                    request_selected_usb_capable(
                        &client,
                        &devd,
                        selector,
                        Method::DELETE,
                        "/wifi",
                        None,
                        "Wi-Fi configuration changes",
                    )
                    .await?
                }
            },
            Command::Ports { selector, command } => {
                handle_ports(&client, &devd, selector, command).await?
            }
            Command::Flash(args) => handle_flash(&client, &devd, args).await?,
            Command::Reset(selector) => {
                let device = resolve_usb_device(&selector, &devd.endpoint)?;
                let device_devd = devd.with_endpoint(device.devd.clone());
                ensure_devd_device_registered(&client, &device_devd, &device.device).await?;
                devd_device_post_with_lease(
                    &client,
                    &device_devd,
                    &device.device,
                    "/reset",
                    json!({}),
                )
                .await?
            }
            Command::Monitor { selector, tail } => {
                let device = resolve_usb_device(&selector, &devd.endpoint)?;
                let device_devd = devd.with_endpoint(device.devd.clone());
                ensure_devd_device_registered(&client, &device_devd, &device.device).await?;
                devd_request(
                    &client,
                    &device_devd,
                    Method::GET,
                    &format!("/api/v1/devices/{}/session?tail={tail}", device.device),
                    None,
                )
                .await?
            }
            Command::Diagnostics { command } => match command {
                DiagnosticsCommand::Export(selector) => {
                    request_selected(&client, &devd, selector, Method::GET, "/diagnostics", None)
                        .await?
                }
            },
        })
    }
    .await;
    let value = match value_result {
        Ok(value) => value,
        Err(err) if err.downcast_ref::<UserCancelled>().is_some() => return Ok(()),
        Err(err) => return Err(err),
    };

    ensure_success_envelope(&value)?;
    let output = redact_sensitive(&value);
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(())
}
