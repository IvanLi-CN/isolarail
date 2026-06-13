use core::fmt::Write as _;

use heapless::String;

pub const DEVICE_VARIANT: &str = "v3";
pub const FIRMWARE_NAME: &str = "iso-usb-hub";
pub const HOSTNAME_PREFIX: &str = "isohub-";

/// Derive a 6-character lowercase hex short ID from the last 3 bytes of the MAC.
pub fn short_id_from_mac(mac: [u8; 6]) -> String<6> {
    let mut out: String<6> = String::new();
    for byte in mac.iter().skip(3) {
        let _ = write!(out, "{:02x}", byte);
    }
    out
}

/// Build the hostname (`isohub-<short_id>`) from the provided short ID.
pub fn hostname_from_short_id(short_id: &str) -> String<32> {
    let mut out: String<32> = String::new();
    let _ = out.push_str(HOSTNAME_PREFIX);
    for ch in short_id.chars() {
        if ch.is_ascii_alphanumeric() {
            let _ = out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

/// Append `.local` to the hostname.
pub fn fqdn_from_hostname(hostname: &str) -> String<48> {
    let mut out: String<48> = String::new();
    let _ = out.push_str(hostname);
    let _ = out.push_str(".local");
    out
}

pub fn mac_to_string(mac: [u8; 6]) -> String<17> {
    let mut out: String<17> = String::new();
    let _ = write!(
        out,
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_id_from_mac_basic_cases() {
        assert_eq!(
            short_id_from_mac([0x00, 0x11, 0x22, 0xAA, 0xBB, 0xCC]).as_str(),
            "aabbcc"
        );
        assert_eq!(short_id_from_mac([0, 0, 0, 0, 0, 0]).as_str(), "000000");
        assert_eq!(short_id_from_mac([0xFF; 6]).as_str(), "ffffff");
    }

    #[test]
    fn hostname_and_fqdn_are_built_correctly() {
        let h = hostname_from_short_id("aabbcc");
        assert_eq!(h.as_str(), "isohub-aabbcc");
        let fqdn = fqdn_from_hostname(h.as_str());
        assert_eq!(fqdn.as_str(), "isohub-aabbcc.local");
    }

    #[test]
    fn mac_to_string_formats_lower_hex_pairs() {
        assert_eq!(
            mac_to_string([0x02, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE]).as_str(),
            "02:aa:bb:cc:dd:ee"
        );
    }
}
