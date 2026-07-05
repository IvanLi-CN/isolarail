use iso_usb_hub_host::{
    device_list, device_status, format_snapshot_tree, format_status, read_snapshot_source,
    HostError, LOCAL_DEVICE_ID,
};
use std::thread;
use std::time::Duration;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), HostError> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let json = take_flag(&mut args, "--json");
    let watch = take_flag(&mut args, "--watch");
    let sample = take_flag(&mut args, "--sample");
    let snapshot_file = take_option(&mut args, "--snapshot-file");

    match args.as_slice() {
        [cmd, sub] if cmd == "devices" && sub == "list" => {
            let snapshot = read_snapshot_source(snapshot_file.as_deref(), sample)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&device_list(&snapshot)).unwrap()
                );
            } else {
                println!(
                    "{LOCAL_DEVICE_ID}\tstate={}",
                    snapshot
                        .pointer("/boot/outcome")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                );
            }
        }
        [cmd, id, sub] if cmd == "device" && sub == "status" => {
            require_local_device(id)?;
            let snapshot = read_snapshot_source(snapshot_file.as_deref(), sample)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&device_status(&snapshot)).unwrap()
                );
            } else {
                println!("{}", format_status(&snapshot));
            }
        }
        [cmd, id, sub] if cmd == "device" && sub == "diag-snapshot" => {
            require_local_device(id)?;
            loop {
                let snapshot = read_snapshot_source(snapshot_file.as_deref(), sample)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
                } else {
                    print!("{}", format_snapshot_tree(&snapshot));
                }
                if !watch {
                    break;
                }
                thread::sleep(Duration::from_secs(2));
            }
        }
        _ => {
            print_usage();
            return Err(HostError::BadRequest("invalid command".to_string()));
        }
    }
    Ok(())
}

fn require_local_device(id: &str) -> Result<(), HostError> {
    if id == LOCAL_DEVICE_ID {
        Ok(())
    } else {
        Err(HostError::BadRequest(format!(
            "unknown device '{id}', only '{LOCAL_DEVICE_ID}' is available in source-built devd mode"
        )))
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

fn print_usage() {
    eprintln!(
        "usage:
  iso-usb-hub devices list [--json] [--snapshot-file <path>|--sample]
  iso-usb-hub device local status [--json] [--snapshot-file <path>|--sample]
  iso-usb-hub device local diag-snapshot [--watch] [--json] [--snapshot-file <path>|--sample]"
    );
}
