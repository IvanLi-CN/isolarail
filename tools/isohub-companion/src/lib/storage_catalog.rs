pub fn registry_path() -> anyhow::Result<PathBuf> {
    let dirs = ProjectDirs::from("cc", "isohub", "isohub")
        .ok_or_else(|| anyhow!("cannot resolve user config directory"))?;
    Ok(dirs.config_dir().join(STORAGE_FILE_NAME))
}

pub fn read_hardware_registry() -> anyhow::Result<HardwareRegistry> {
    let path = registry_path()?;
    let raw = match fs::read(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HardwareRegistry {
                schema_version: STORAGE_SCHEMA_VERSION,
                devices: Vec::new(),
            });
        }
        Err(err) => return Err(err).context("read hardware registry"),
    };
    let mut registry: HardwareRegistry = serde_json::from_slice(&raw)?;
    if registry.schema_version == 0 {
        registry.schema_version = STORAGE_SCHEMA_VERSION;
    }
    Ok(registry)
}

pub fn write_hardware_registry(registry: &HardwareRegistry) -> anyhow::Result<()> {
    let path = registry_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(registry)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn settings_path() -> anyhow::Result<PathBuf> {
    let dirs = ProjectDirs::from("cc", "isohub", "isohub")
        .ok_or_else(|| anyhow!("cannot resolve user config directory"))?;
    Ok(dirs.config_dir().join(STORAGE_SETTINGS_FILE_NAME))
}

fn default_storage_settings() -> StorageSettings {
    StorageSettings {
        theme: "isohub".to_string(),
    }
}

fn read_storage_settings() -> anyhow::Result<StorageSettings> {
    let path = settings_path()?;
    let raw = match fs::read(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(default_storage_settings());
        }
        Err(err) => return Err(err).context("read storage settings"),
    };
    Ok(serde_json::from_slice(&raw)?)
}

fn write_storage_settings(settings: &StorageSettings) -> anyhow::Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn save_hardware(input: SavedHardwareInput) -> anyhow::Result<DeviceProfile> {
    let profiles = save_hardware_profiles(vec![input])?;
    profiles
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no hardware profile saved"))
}

pub fn save_hardware_profiles(inputs: Vec<SavedHardwareInput>) -> anyhow::Result<Vec<DeviceProfile>> {
    let mut registry = read_hardware_registry()?;
    let now = now_unix_seconds();
    let mut saved = Vec::with_capacity(inputs.len());
    for input in inputs {
        let profile = DeviceProfile {
            id: input.id,
            name: input.name,
            transport: input.transport,
            identity: input.identity,
            last_seen_at: Some(now),
        };
        upsert_profile(&mut registry, profile.clone());
        saved.push(profile);
    }
    write_hardware_registry(&registry)?;
    Ok(saved)
}

fn delete_hardware(id: &str) -> anyhow::Result<bool> {
    let mut registry = read_hardware_registry()?;
    let before = registry.devices.len();
    registry.devices.retain(|device| device.id != id);
    write_hardware_registry(&registry)?;
    Ok(before != registry.devices.len())
}

fn import_profiles(profiles: Vec<DeviceProfile>) -> anyhow::Result<usize> {
    let mut registry = read_hardware_registry()?;
    let mut count = 0;
    for mut profile in profiles {
        if profile.last_seen_at.is_none() {
            profile.last_seen_at = Some(now_unix_seconds());
        }
        upsert_profile(&mut registry, profile);
        count += 1;
    }
    write_hardware_registry(&registry)?;
    Ok(count)
}

fn upsert_profile(registry: &mut HardwareRegistry, profile: DeviceProfile) {
    if let Some(existing) = registry
        .devices
        .iter_mut()
        .find(|device| device.id == profile.id)
    {
        let mut profile = profile;
        if profile.identity.is_none() {
            profile.identity = existing.identity.clone();
        }
        *existing = profile;
    } else {
        registry.devices.push(profile);
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn api_url(base: &str, path: &str) -> anyhow::Result<reqwest::Url> {
    let base = reqwest::Url::parse(base)?;
    Ok(base.join(path.trim_start_matches('/'))?)
}

pub fn validate_catalog_shape(catalog: &FirmwareCatalog) -> Vec<String> {
    let mut errors = Vec::new();
    if catalog.schema_version.trim().is_empty() {
        errors.push("schemaVersion is required".to_string());
    }
    if catalog.artifacts.is_empty() {
        errors.push("at least one artifact is required".to_string());
    }
    for artifact in &catalog.artifacts {
        if artifact.artifact_id.trim().is_empty() {
            errors.push("artifactId is required".to_string());
        }
        if artifact.target != "esp32s3_app" && artifact.target != "esp32s3_full" {
            errors.push(format!("unsupported target {}", artifact.target));
        }
        for file in &artifact.files {
            if file.kind == "app_bin" && file.flash_address != Some(DEFAULT_FLASH_ADDRESS) {
                errors.push(format!(
                    "app_bin {} must use flashAddress 0x10000",
                    file.path
                ));
            }
            if file.kind == "full_image" && file.flash_address.is_some_and(|address| address != 0) {
                errors.push(format!(
                    "full_image {} must use flashAddress 0x0",
                    file.path
                ));
            }
            if file.sha256.len() != 64 || !file.sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
                errors.push(format!("file {} has invalid sha256", file.path));
            }
        }
    }
    errors
}

fn verify_artifact_file(catalog_path: &FsPath, file: &FirmwareFile) -> anyhow::Result<()> {
    let file_path = resolve_catalog_file_path(catalog_path, &file.path);
    let bytes = fs::read(&file_path).with_context(|| format!("read {}", file_path.display()))?;
    if bytes.len() as u64 != file.size {
        return Err(anyhow!(
            "artifact size mismatch for {}: expected {}, got {}",
            file.path,
            file.size,
            bytes.len()
        ));
    }
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != file.sha256.to_lowercase() {
        return Err(anyhow!(
            "artifact hash mismatch for {}: expected {}, got {actual}",
            file.path,
            file.sha256
        ));
    }
    Ok(())
}

fn resolve_catalog_file_path(catalog_path: &FsPath, relative: &str) -> PathBuf {
    let path = FsPath::new(relative);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    catalog_path
        .parent()
        .unwrap_or_else(|| FsPath::new("."))
        .join(path)
}

async fn require_compatible_project_firmware(
    state: &AppState,
    device_id: &str,
) -> anyhow::Result<Value> {
    let info = match usb_jsonl_request(state, device_id, "info", None).await {
        Ok(info) => info,
        Err(err) => {
            return Err(anyhow!(
                "device did not respond to IsoHub `info`; it may be in download mode or running non-project firmware: {err:#}"
            ));
        }
    };
    validate_project_firmware(&info)?;
    Ok(info)
}

fn validate_project_firmware(info: &Value) -> anyhow::Result<()> {
    let firmware = project_firmware_metadata(info)?;
    validate_project_firmware_name(firmware)?;
    let version = firmware
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("info response did not include firmware.version"))?;
    if !version_at_least(version, MIN_COMPATIBLE_FIRMWARE_VERSION) {
        return Err(anyhow!(
            "connected device firmware version `{version}` is incompatible; upgrade firmware to `{MIN_COMPATIBLE_FIRMWARE_VERSION}` or newer"
        ));
    }
    Ok(())
}

async fn require_project_firmware_for_upgrade(
    state: &AppState,
    device_id: &str,
) -> anyhow::Result<Value> {
    let info = match usb_jsonl_request(state, device_id, "info", None).await {
        Ok(info) => info,
        Err(err) => {
            return Err(anyhow!(
                "device did not respond to IsoHub `info`; it may be in download mode or running non-project firmware: {err:#}"
            ));
        }
    };
    let firmware = project_firmware_metadata(&info)?;
    validate_project_firmware_name(firmware)?;
    Ok(info)
}

fn project_firmware_metadata(info: &Value) -> anyhow::Result<&Value> {
    let device = info
        .get("result")
        .and_then(|value| value.get("device"))
        .or_else(|| info.get("device"))
        .ok_or_else(|| anyhow!("info response did not include device identity"))?;
    device
        .get("firmware")
        .ok_or_else(|| anyhow!("info response did not include firmware metadata"))
}

fn validate_project_firmware_name(firmware: &Value) -> anyhow::Result<()> {
    let name = firmware
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("info response did not include firmware.name"))?;
    if name != PROJECT_FIRMWARE_NAME {
        return Err(anyhow!(
            "connected device is running firmware `{name}`, expected `{PROJECT_FIRMWARE_NAME}`; refusing operation"
        ));
    }
    Ok(())
}

fn version_at_least(actual: &str, minimum: &str) -> bool {
    let actual = parse_version_triplet(actual);
    let minimum = parse_version_triplet(minimum);
    matches!((actual, minimum), (Some(actual), Some(minimum)) if actual >= minimum)
}

fn parse_version_triplet(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn validate_device_identity(info: &Value, expected: &DeviceIdentity) -> anyhow::Result<()> {
    if expected.device_id.is_none() && expected.mac.is_none() {
        return Err(anyhow!("expectedIdentity must include deviceId or mac"));
    }
    let device = info
        .get("result")
        .and_then(|value| value.get("device"))
        .or_else(|| info.get("device"))
        .ok_or_else(|| anyhow!("info response did not include device identity"))?;
    if let Some(expected_id) = expected.device_id.as_deref() {
        let actual_id = device
            .get("device_id")
            .or_else(|| device.get("deviceId"))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("info response did not include device_id"))?;
        if actual_id != expected_id {
            return Err(anyhow!(
                "device identity mismatch: expected device_id {expected_id}, got {actual_id}"
            ));
        }
    }
    if let Some(expected_mac) = expected.mac.as_deref() {
        let actual_mac = device
            .get("mac")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("info response did not include mac"))?;
        if !actual_mac.eq_ignore_ascii_case(expected_mac) {
            return Err(anyhow!(
                "device identity mismatch: expected mac {expected_mac}, got {actual_mac}"
            ));
        }
    }
    Ok(())
}

async fn capture_first_time_identity_after_flash(
    state: &AppState,
    device_id: &str,
) -> anyhow::Result<DeviceIdentity> {
    let mut last_error: Option<anyhow::Error> = None;
    for _ in 0..15 {
        tokio::time::sleep(Duration::from_millis(1_000)).await;
        match usb_jsonl_request_with_exclusive(
            state,
            device_id,
            "info",
            None,
            Some("firmware flash"),
        )
        .await
        {
            Ok(info) => {
                let identity = extract_device_identity(&info)?;
                persist_captured_identity(state, device_id, &identity).await?;
                return Ok(identity);
            }
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("first-time identity capture failed")))
}

fn extract_device_identity(info: &Value) -> anyhow::Result<DeviceIdentity> {
    let device = info
        .get("result")
        .and_then(|value| value.get("device"))
        .or_else(|| info.get("device"))
        .ok_or_else(|| anyhow!("info response did not include device identity"))?;
    let identity = DeviceIdentity {
        device_id: device
            .get("device_id")
            .or_else(|| device.get("deviceId"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        mac: device
            .get("mac")
            .and_then(Value::as_str)
            .map(ToString::to_string),
    };
    if identity.device_id.is_none() && identity.mac.is_none() {
        return Err(anyhow!("info response did not include device_id or mac"));
    }
    Ok(identity)
}

async fn persist_captured_identity(
    state: &AppState,
    device_id: &str,
    identity: &DeviceIdentity,
) -> anyhow::Result<()> {
    {
        let mut inner = state.inner.lock().await;
        if let Some(device) = inner.devices.get_mut(device_id) {
            device.identity = Some(serde_json::to_value(identity)?);
        }
    }

    let mut registry = read_hardware_registry()?;
    let mut changed = false;
    for profile in &mut registry.devices {
        if let HardwareTransport::Usb {
            device_id: saved_device_id,
            ..
        } = &profile.transport
            && saved_device_id == device_id
        {
            profile.identity = Some(identity.clone());
            profile.last_seen_at = Some(now_unix_seconds());
            changed = true;
        }
    }
    if changed {
        write_hardware_registry(&registry)?;
    }
    Ok(())
}

pub fn redact_sensitive(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let lower = key.to_lowercase();
                    if matches!(
                        lower.as_str(),
                        "psk" | "password" | "passphrase" | "secret" | "token"
                    ) {
                        (key.clone(), Value::String("<redacted>".to_string()))
                    } else {
                        (key.clone(), redact_sensitive(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_sensitive).collect()),
        other => other.clone(),
    }
}

async fn push_trace(
    state: &AppState,
    device_id: &str,
    level: &str,
    message: &str,
    payload: &Value,
) {
    let mut inner = state.inner.lock().await;
    if let Some(device) = inner.devices.get_mut(device_id) {
        bounded_push(
            &mut device.session.traces,
            SessionItem {
                id: next_id(),
                timestamp_unix_ms: now_unix_millis(),
                level: level.to_string(),
                message: message.to_string(),
                payload: redact_sensitive(payload),
            },
        );
    }
}

async fn push_log(
    state: &AppState,
    device_id: &str,
    level: &str,
    message: &str,
    payload: &Value,
) {
    let mut inner = state.inner.lock().await;
    if let Some(device) = inner.devices.get_mut(device_id) {
        bounded_push(
            &mut device.session.logs,
            SessionItem {
                id: next_id(),
                timestamp_unix_ms: now_unix_millis(),
                level: level.to_string(),
                message: message.to_string(),
                payload: redact_sensitive(payload),
            },
        );
    }
}

fn bounded_push(items: &mut VecDeque<SessionItem>, item: SessionItem) {
    while items.len() >= MAX_SESSION_ITEMS {
        items.pop_front();
    }
    items.push_back(item);
}

fn tail_items(items: &VecDeque<SessionItem>, tail: usize) -> Vec<SessionItem> {
    items
        .iter()
        .skip(items.len().saturating_sub(tail))
        .cloned()
        .collect()
}

fn now_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn next_id() -> String {
    format!("{:x}", now_unix_millis()) + "-" + &generate_token()[..8]
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

fn require_auth(headers: &HeaderMap, state: &AppState) -> Result<(), Box<Response>> {
    let Some(auth) = headers.get(header::AUTHORIZATION) else {
        return Err(Box::new(unauthorized("missing bearer token")));
    };
    let Ok(auth) = auth.to_str() else {
        return Err(Box::new(unauthorized("invalid bearer token")));
    };
    if auth != format!("Bearer {}", state.token) {
        return Err(Box::new(unauthorized("invalid bearer token")));
    }
    Ok(())
}

fn is_loopback_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(url) = reqwest::Url::parse(origin) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https" | "tauri")
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "::1"))
}

fn error_from_anyhow(err: anyhow::Error) -> Response {
    let message = err.to_string();
    if message.contains("busy") {
        conflict(&err.to_string())
    } else if message.contains("non-project firmware")
        || message.contains("not include firmware")
        || message.contains("firmware version")
        || message.contains("expected `iso-usb-hub`")
        || message.contains("did not respond to IsoHub")
    {
        bad_request(&message)
    } else {
        internal_error(&message)
    }
}

fn unauthorized(message: &str) -> Response {
    error_response(StatusCode::UNAUTHORIZED, "unauthorized", message, false)
}

fn bad_request(message: &str) -> Response {
    error_response(StatusCode::BAD_REQUEST, "bad_request", message, false)
}

fn not_found(message: &str) -> Response {
    error_response(StatusCode::NOT_FOUND, "not_found", message, false)
}

fn conflict(message: &str) -> Response {
    error_response(StatusCode::CONFLICT, "busy", message, true)
}

fn internal_error(message: &str) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        message,
        false,
    )
}

fn error_response(
    status: StatusCode,
    code: &'static str,
    message: &str,
    retryable: bool,
) -> Response {
    (
        status,
        Json(ErrorEnvelope {
            error: ErrorInfo {
                code,
                message: message.to_string(),
                retryable,
            },
        }),
    )
        .into_response()
}
