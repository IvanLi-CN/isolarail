use iso_usb_hub_host::{
    device_list, device_status, http_response, read_snapshot_source, HostError,
};
use serde_json::json;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), HostError> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let sample = take_flag(&mut args, "--sample");
    let snapshot_file = take_option(&mut args, "--snapshot-file");
    match args.as_slice() {
        [cmd] if cmd == "serve" => {
            serve_ipc(snapshot_file.as_deref(), sample, default_socket_path())?
        }
        [cmd, socket_flag, socket] if cmd == "serve" && socket_flag == "--socket" => {
            serve_ipc(snapshot_file.as_deref(), sample, PathBuf::from(socket))?
        }
        [cmd] if cmd == "bridge-http" => {
            serve_http(snapshot_file.as_deref(), sample, "127.0.0.1:51210")?
        }
        [cmd, bind_flag, bind] if cmd == "bridge-http" && bind_flag == "--bind" => {
            serve_http(snapshot_file.as_deref(), sample, bind)?
        }
        _ => {
            print_usage();
            return Err(HostError::BadRequest("invalid command".to_string()));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn serve_ipc(
    snapshot_file: Option<&str>,
    sample: bool,
    socket_path: PathBuf,
) -> Result<(), HostError> {
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixListener;

    if let Ok(metadata) = std::fs::symlink_metadata(&socket_path) {
        if !metadata.file_type().is_socket() {
            return Err(HostError::BadRequest(format!(
                "refusing to remove non-socket path: {}",
                socket_path.display()
            )));
        }
        std::fs::remove_file(&socket_path)?;
    }
    let listener = UnixListener::bind(&socket_path)?;
    eprintln!(
        "iso-usb-hub-devd: ipc listening on {}",
        socket_path.display()
    );
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_ipc_stream(stream, snapshot_file, sample) {
                    eprintln!("iso-usb-hub-devd: ipc request failed: {err}");
                }
            }
            Err(err) => eprintln!("iso-usb-hub-devd: ipc accept failed: {err}"),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn handle_ipc_stream(
    mut stream: UnixStream,
    snapshot_file: Option<&str>,
    sample: bool,
) -> Result<(), HostError> {
    let cloned = stream.try_clone()?;
    let mut reader = BufReader::new(cloned);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let snapshot = read_snapshot_source(snapshot_file, sample)?;
    let body = if line.contains("devices.list") {
        device_list(&snapshot).to_string()
    } else if line.contains("device.status") {
        device_status(&snapshot).to_string()
    } else {
        serde_json::to_string(&snapshot).unwrap()
    };
    stream.write_all(body.as_bytes())?;
    stream.write_all(b"\n")?;
    Ok(())
}

#[cfg(not(unix))]
fn serve_ipc(
    _snapshot_file: Option<&str>,
    _sample: bool,
    _socket_path: PathBuf,
) -> Result<(), HostError> {
    Err(HostError::BadRequest(
        "local IPC is implemented with Unix sockets on this host build".to_string(),
    ))
}

fn serve_http(snapshot_file: Option<&str>, sample: bool, bind: &str) -> Result<(), HostError> {
    let listener = TcpListener::bind(bind)?;
    eprintln!("iso-usb-hub-devd: http bridge listening on http://{bind}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_http_stream(stream, snapshot_file, sample) {
                    eprintln!("iso-usb-hub-devd: http request failed: {err}");
                }
            }
            Err(err) => eprintln!("iso-usb-hub-devd: http accept failed: {err}"),
        }
    }
    Ok(())
}

fn handle_http_stream(
    mut stream: TcpStream,
    snapshot_file: Option<&str>,
    sample: bool,
) -> Result<(), HostError> {
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut parts = req.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let (status, content_type, body) = match read_snapshot_source(snapshot_file, sample) {
        Ok(snapshot) => http_response(method, path, &snapshot),
        Err(err) => (
            500,
            "application/json",
            json!({"error":"snapshot_unavailable","message":err.to_string()}).to_string(),
        ),
    };
    let reason = reason_phrase(status);
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )?;
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Error",
    }
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let found = args.iter().position(|arg| arg == flag);
    if let Some(idx) = found {
        args.remove(idx);
        true
    } else {
        false
    }
}

fn take_option(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let idx = args.iter().position(|arg| arg == flag)?;
    args.remove(idx);
    if idx < args.len() {
        Some(args.remove(idx))
    } else {
        None
    }
}

fn default_socket_path() -> PathBuf {
    std::env::temp_dir().join("iso-usb-hub-devd.sock")
}

fn print_usage() {
    eprintln!(
        "usage:
  iso-usb-hub-devd serve [--socket <path>] [--snapshot-file <path>|--sample]
  iso-usb-hub-devd bridge-http [--bind 127.0.0.1:51210] [--snapshot-file <path>|--sample]"
    );
}
