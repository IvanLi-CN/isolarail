#![allow(dead_code)]

use defmt::*;
use embassy_futures::select::{select, Either};
use embassy_net::{
    udp::{PacketMetadata, RecvError, SendError, UdpSocket},
    IpAddress, IpEndpoint, Ipv4Address, Stack,
};
use embassy_time::{Duration, Timer};
use heapless::String;

const MDNS_MULTICAST_V4: Ipv4Address = Ipv4Address::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const MDNS_RESPONSE_TTL_SECS: u32 = 120;
const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_DELAY: Duration = Duration::from_secs(2);

const HTTP_SERVICE_TYPE: &str = "_http._tcp.local";
const HTTP_TXT_PATH: &str = "path=/";

/// Configuration for the mDNS task.
#[derive(Clone)]
pub struct MdnsConfig {
    /// Hostname without the `.local` suffix, e.g. `isolarail-a1b2c3`.
    pub hostname: String<32>,
    /// Fully qualified `.local` hostname, e.g. `isolarail-a1b2c3.local`.
    pub hostname_fqdn: String<48>,
    /// Service instance name, e.g. `isolarail-a1b2c3._http._tcp.local`.
    pub instance_name: String<64>,
    /// HTTP service port (currently 80).
    pub port: u16,
}

/// Build the HTTP service instance name (`<hostname>._http._tcp.local`).
pub fn service_instance_name(hostname: &str) -> String<64> {
    let mut out: String<64> = String::new();
    let _ = out.push_str(hostname);
    let _ = out.push_str(".");
    let _ = out.push_str(HTTP_SERVICE_TYPE);
    out
}

#[embassy_executor::task]
pub async fn mdns_task(stack: Stack<'static>, cfg: MdnsConfig) {
    run_mdns(stack, cfg).await;
}

pub async fn run_mdns(stack: Stack<'static>, cfg: MdnsConfig) -> ! {
    loop {
        stack.wait_config_up().await;

        let ip = match stack.config_v4() {
            Some(v4) => v4.address.address(),
            None => {
                Timer::after(RETRY_DELAY).await;
                continue;
            }
        };

        if let Err(err) = stack.join_multicast_group(IpAddress::Ipv4(MDNS_MULTICAST_V4)) {
            warn!(
                "mdns: failed to join multicast group (hostname={}): {:?}",
                cfg.hostname_fqdn.as_str(),
                err
            );
            Timer::after(RETRY_DELAY).await;
            continue;
        }

        let mut rx_meta = [PacketMetadata::EMPTY; 4];
        let mut tx_meta = [PacketMetadata::EMPTY; 4];
        let mut rx_storage = [0u8; 512];
        let mut tx_storage = [0u8; 512];
        let mut recv_buf = [0u8; 512];
        let mut resp_buf = [0u8; 512];

        let mut socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_storage,
            &mut tx_meta,
            &mut tx_storage,
        );
        socket.set_hop_limit(Some(255));

        // Binding to the current IPv4 address (instead of 0.0.0.0) avoids emitting responses
        // with a source address of 0.0.0.0, which some resolvers will drop.
        if let Err(err) = socket.bind((IpAddress::Ipv4(ip), MDNS_PORT)) {
            warn!(
                "mdns: bind 5353 failed (hostname={}): {:?}",
                cfg.hostname_fqdn.as_str(),
                err
            );
            Timer::after(RETRY_DELAY).await;
            continue;
        }

        info!(
            "mdns: announcing HTTP service (hostname={}, ip={}, port={})",
            cfg.hostname_fqdn.as_str(),
            ip,
            cfg.port
        );

        // Initial unsolicited announcement.
        send_announce(&mut socket, &mut resp_buf, &cfg, ip).await;

        let mut announce_timer = Timer::after(ANNOUNCE_INTERVAL);

        loop {
            let recv_fut = socket.recv_from(&mut recv_buf);
            match select(recv_fut, announce_timer).await {
                Either::First(res) => {
                    announce_timer = Timer::after(ANNOUNCE_INTERVAL);
                    match res {
                        Ok((len, meta)) => {
                            if let Some(query) = parse_query(&recv_buf[..len]) {
                                if let Some(kind) = classify_query(&query, &cfg) {
                                    let dest = if query.unicast_response {
                                        meta.endpoint
                                    } else {
                                        IpEndpoint::new(
                                            IpAddress::Ipv4(MDNS_MULTICAST_V4),
                                            MDNS_PORT,
                                        )
                                    };
                                    send_query_response(
                                        &mut socket,
                                        &mut resp_buf,
                                        &cfg,
                                        ip,
                                        &query,
                                        kind,
                                        dest,
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(err) => match err {
                            RecvError::Truncated => warn!("mdns: truncated datagram"),
                        },
                    }
                }
                Either::Second(_) => {
                    // Periodic unsolicited announcement.
                    send_announce(&mut socket, &mut resp_buf, &cfg, ip).await;
                    announce_timer = Timer::after(ANNOUNCE_INTERVAL);
                }
            }

            if !stack.is_config_up() {
                break;
            }
        }

        Timer::after(RETRY_DELAY).await;
    }
}

#[derive(Clone, Copy, Debug)]
enum ResponseKind {
    AOnly,
    Service,  // PTR + (SRV/TXT/A in additional)
    Instance, // SRV+TXT + (A in additional)
}

fn classify_query(query: &Query<'_>, cfg: &MdnsConfig) -> Option<ResponseKind> {
    let qtype = query.qtype;
    let name = query.name.as_str();

    let is_any = qtype == 255;
    if name_matches(name, cfg.hostname_fqdn.as_str()) && (qtype == 1 || is_any) {
        return Some(ResponseKind::AOnly);
    }
    if name_matches(name, HTTP_SERVICE_TYPE) && (qtype == 12 || is_any) {
        return Some(ResponseKind::Service);
    }
    if name_matches(name, cfg.instance_name.as_str()) && (qtype == 33 || qtype == 16 || is_any) {
        return Some(ResponseKind::Instance);
    }

    None
}

async fn send_announce(
    socket: &mut UdpSocket<'_>,
    buf: &mut [u8],
    cfg: &MdnsConfig,
    ip: Ipv4Address,
) {
    let len = build_announce(buf, cfg, ip).unwrap_or_else(|| {
        warn!("mdns: failed to encode announce (buffer too small)");
        0
    });
    if len == 0 {
        return;
    }
    let dest = IpEndpoint::new(IpAddress::Ipv4(MDNS_MULTICAST_V4), MDNS_PORT);
    if let Err(err) = socket.send_to(&buf[..len], dest).await {
        match err {
            SendError::NoRoute => warn!("mdns: announce send_to no route"),
            SendError::SocketNotBound => warn!("mdns: announce socket not bound"),
            SendError::PacketTooLarge => warn!("mdns: announce packet too large"),
        }
    }
}

async fn send_query_response(
    socket: &mut UdpSocket<'_>,
    buf: &mut [u8],
    cfg: &MdnsConfig,
    ip: Ipv4Address,
    query: &Query<'_>,
    kind: ResponseKind,
    dest: IpEndpoint,
) {
    let len = match kind {
        ResponseKind::AOnly => build_a_only_response(buf, cfg, ip, Some(query)),
        ResponseKind::Service => build_service_response(buf, cfg, ip, Some(query)),
        ResponseKind::Instance => build_instance_response(buf, cfg, ip, Some(query)),
    }
    .unwrap_or_else(|| {
        warn!("mdns: failed to encode response (buffer too small)");
        0
    });
    if len == 0 {
        return;
    }

    if let Err(err) = socket.send_to(&buf[..len], dest).await {
        match err {
            SendError::NoRoute => warn!("mdns: send_to no route"),
            SendError::SocketNotBound => warn!("mdns: socket not bound"),
            SendError::PacketTooLarge => warn!("mdns: packet too large"),
        }
    }
}

fn build_announce(buf: &mut [u8], cfg: &MdnsConfig, ip: Ipv4Address) -> Option<usize> {
    // Unsolicited announcement: answers contain PTR + SRV + TXT + A.
    let qdcount = 0u16;
    let ancount = 4u16;
    write_header(buf, qdcount, ancount, 0, 0)?;

    let mut offset = 12usize;
    offset = write_ptr_record(buf, offset, HTTP_SERVICE_TYPE, cfg.instance_name.as_str())?;
    offset = write_srv_record(
        buf,
        offset,
        cfg.instance_name.as_str(),
        cfg.port,
        cfg.hostname_fqdn.as_str(),
    )?;
    offset = write_txt_record(buf, offset, cfg.instance_name.as_str(), HTTP_TXT_PATH)?;
    offset = write_a_record(buf, offset, cfg.hostname_fqdn.as_str(), ip)?;
    Some(offset)
}

fn build_a_only_response(
    buf: &mut [u8],
    cfg: &MdnsConfig,
    ip: Ipv4Address,
    query: Option<&Query<'_>>,
) -> Option<usize> {
    let qdcount = if query.is_some() { 1 } else { 0 };
    let ancount = 1u16;
    write_header(buf, qdcount, ancount, 0, 0)?;

    let mut offset = 12usize;
    if let Some(q) = query {
        offset = write_question(buf, offset, q)?;
    }
    offset = write_a_record(buf, offset, cfg.hostname_fqdn.as_str(), ip)?;
    Some(offset)
}

fn build_service_response(
    buf: &mut [u8],
    cfg: &MdnsConfig,
    ip: Ipv4Address,
    query: Option<&Query<'_>>,
) -> Option<usize> {
    let qdcount = if query.is_some() { 1 } else { 0 };
    let ancount = 1u16; // PTR
    let arcount = 3u16; // SRV + TXT + A
    write_header(buf, qdcount, ancount, 0, arcount)?;

    let mut offset = 12usize;
    if let Some(q) = query {
        offset = write_question(buf, offset, q)?;
    }

    // Answer: service PTR.
    offset = write_ptr_record(buf, offset, HTTP_SERVICE_TYPE, cfg.instance_name.as_str())?;
    // Additional: SRV, TXT, A.
    offset = write_srv_record(
        buf,
        offset,
        cfg.instance_name.as_str(),
        cfg.port,
        cfg.hostname_fqdn.as_str(),
    )?;
    offset = write_txt_record(buf, offset, cfg.instance_name.as_str(), HTTP_TXT_PATH)?;
    offset = write_a_record(buf, offset, cfg.hostname_fqdn.as_str(), ip)?;

    Some(offset)
}

fn build_instance_response(
    buf: &mut [u8],
    cfg: &MdnsConfig,
    ip: Ipv4Address,
    query: Option<&Query<'_>>,
) -> Option<usize> {
    let qdcount = if query.is_some() { 1 } else { 0 };
    let ancount = 2u16; // SRV + TXT
    let arcount = 1u16; // A
    write_header(buf, qdcount, ancount, 0, arcount)?;

    let mut offset = 12usize;
    if let Some(q) = query {
        offset = write_question(buf, offset, q)?;
    }

    offset = write_srv_record(
        buf,
        offset,
        cfg.instance_name.as_str(),
        cfg.port,
        cfg.hostname_fqdn.as_str(),
    )?;
    offset = write_txt_record(buf, offset, cfg.instance_name.as_str(), HTTP_TXT_PATH)?;
    offset = write_a_record(buf, offset, cfg.hostname_fqdn.as_str(), ip)?;
    Some(offset)
}

fn write_header(
    buf: &mut [u8],
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
) -> Option<()> {
    if buf.len() < 12 {
        return None;
    }
    // ID = 0
    buf[0] = 0;
    buf[1] = 0;
    // QR=1, AA=1
    buf[2] = 0x84;
    buf[3] = 0x00;
    buf[4..6].copy_from_slice(&qdcount.to_be_bytes());
    buf[6..8].copy_from_slice(&ancount.to_be_bytes());
    buf[8..10].copy_from_slice(&nscount.to_be_bytes());
    buf[10..12].copy_from_slice(&arcount.to_be_bytes());
    Some(())
}

fn write_question(buf: &mut [u8], offset: usize, q: &Query<'_>) -> Option<usize> {
    let mut offset = encode_dotted_name(buf, offset, q.name.as_str())?;
    if offset + 4 > buf.len() {
        return None;
    }
    buf[offset..offset + 2].copy_from_slice(&q.qtype.to_be_bytes());
    // QCLASS IN (strip unicast bit)
    buf[offset + 2] = 0;
    buf[offset + 3] = 1;
    offset += 4;
    Some(offset)
}

fn write_a_record(buf: &mut [u8], offset: usize, name: &str, ip: Ipv4Address) -> Option<usize> {
    let mut offset = encode_dotted_name(buf, offset, name)?;
    offset = write_rr_fixed_header(buf, offset, 1, true, 4)?;
    let octets = ip.octets();
    if offset + 4 > buf.len() {
        return None;
    }
    buf[offset..offset + 4].copy_from_slice(&octets);
    Some(offset + 4)
}

fn write_ptr_record(buf: &mut [u8], offset: usize, name: &str, target: &str) -> Option<usize> {
    let mut offset = encode_dotted_name(buf, offset, name)?;

    // TYPE PTR (12), CLASS IN (no cache-flush).
    if offset + 10 > buf.len() {
        return None;
    }
    buf[offset..offset + 2].copy_from_slice(&12u16.to_be_bytes());
    buf[offset + 2] = 0;
    buf[offset + 3] = 1;
    buf[offset + 4..offset + 8].copy_from_slice(&MDNS_RESPONSE_TTL_SECS.to_be_bytes());

    // Reserve RDLENGTH.
    let rdlen_pos = offset + 8;
    offset += 10;

    let rdata_start = offset;
    offset = encode_dotted_name(buf, offset, target)?;
    let rdlen = (offset - rdata_start) as u16;
    buf[rdlen_pos..rdlen_pos + 2].copy_from_slice(&rdlen.to_be_bytes());
    Some(offset)
}

fn write_srv_record(
    buf: &mut [u8],
    offset: usize,
    name: &str,
    port: u16,
    target: &str,
) -> Option<usize> {
    let mut offset = encode_dotted_name(buf, offset, name)?;

    if offset + 10 > buf.len() {
        return None;
    }
    buf[offset..offset + 2].copy_from_slice(&33u16.to_be_bytes()); // SRV
                                                                   // CLASS IN with cache-flush bit.
    buf[offset + 2] = 0x80;
    buf[offset + 3] = 0x01;
    buf[offset + 4..offset + 8].copy_from_slice(&MDNS_RESPONSE_TTL_SECS.to_be_bytes());

    // Reserve RDLENGTH.
    let rdlen_pos = offset + 8;
    offset += 10;

    let rdata_start = offset;
    if offset + 6 > buf.len() {
        return None;
    }
    // priority=0, weight=0, port
    buf[offset..offset + 2].copy_from_slice(&0u16.to_be_bytes());
    buf[offset + 2..offset + 4].copy_from_slice(&0u16.to_be_bytes());
    buf[offset + 4..offset + 6].copy_from_slice(&port.to_be_bytes());
    offset += 6;

    offset = encode_dotted_name(buf, offset, target)?;
    let rdlen = (offset - rdata_start) as u16;
    buf[rdlen_pos..rdlen_pos + 2].copy_from_slice(&rdlen.to_be_bytes());
    Some(offset)
}

fn write_txt_record(buf: &mut [u8], offset: usize, name: &str, txt: &str) -> Option<usize> {
    let mut offset = encode_dotted_name(buf, offset, name)?;

    if offset + 10 > buf.len() {
        return None;
    }
    buf[offset..offset + 2].copy_from_slice(&16u16.to_be_bytes()); // TXT
                                                                   // CLASS IN with cache-flush bit.
    buf[offset + 2] = 0x80;
    buf[offset + 3] = 0x01;
    buf[offset + 4..offset + 8].copy_from_slice(&MDNS_RESPONSE_TTL_SECS.to_be_bytes());

    let txt_bytes = txt.as_bytes();
    if txt_bytes.len() > 255 {
        return None;
    }
    let rdlen: u16 = (1 + txt_bytes.len()) as u16;
    buf[offset + 8..offset + 10].copy_from_slice(&rdlen.to_be_bytes());
    offset += 10;

    if offset + rdlen as usize > buf.len() {
        return None;
    }
    buf[offset] = txt_bytes.len() as u8;
    buf[offset + 1..offset + 1 + txt_bytes.len()].copy_from_slice(txt_bytes);
    Some(offset + 1 + txt_bytes.len())
}

fn write_rr_fixed_header(
    buf: &mut [u8],
    offset: usize,
    rr_type: u16,
    cache_flush: bool,
    rdlen: u16,
) -> Option<usize> {
    if offset + 10 > buf.len() {
        return None;
    }
    buf[offset..offset + 2].copy_from_slice(&rr_type.to_be_bytes());
    let class: u16 = if cache_flush { 0x8001 } else { 0x0001 };
    buf[offset + 2..offset + 4].copy_from_slice(&class.to_be_bytes());
    buf[offset + 4..offset + 8].copy_from_slice(&MDNS_RESPONSE_TTL_SECS.to_be_bytes());
    buf[offset + 8..offset + 10].copy_from_slice(&rdlen.to_be_bytes());
    Some(offset + 10)
}

fn encode_dotted_name(buf: &mut [u8], mut offset: usize, name: &str) -> Option<usize> {
    let name = name.trim_end_matches('.');
    if name.is_empty() {
        return None;
    }
    for label in name.split('.') {
        let len = label.len();
        if len == 0 || len > 63 || offset + 1 + len > buf.len() {
            return None;
        }
        buf[offset] = len as u8;
        offset += 1;
        buf[offset..offset + len].copy_from_slice(label.as_bytes());
        offset += len;
    }
    if offset >= buf.len() {
        return None;
    }
    buf[offset] = 0;
    Some(offset + 1)
}

#[derive(Debug)]
struct Query<'a> {
    name: String<64>,
    qtype: u16,
    unicast_response: bool,
    _marker: core::marker::PhantomData<&'a ()>,
}

fn parse_query(packet: &[u8]) -> Option<Query<'_>> {
    if packet.len() < 12 {
        return None;
    }

    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    if flags & 0x8000 != 0 {
        // Not a query.
        return None;
    }

    let qdcount = u16::from_be_bytes([packet[4], packet[5]]);
    if qdcount == 0 {
        return None;
    }

    let mut offset = 12usize;
    let mut name = String::<64>::new();
    offset = decode_name(packet, offset, &mut name)?;

    if offset + 4 > packet.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
    let qclass_raw = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]);
    let unicast = (qclass_raw & 0x8000) != 0;
    let qclass = qclass_raw & 0x7FFF;

    // Accept A/PTR/TXT/SRV/ANY.
    match qtype {
        1 | 12 | 16 | 33 | 255 => {}
        _ => return None,
    }
    if qclass != 1 {
        return None;
    }

    Some(Query {
        name,
        qtype,
        unicast_response: unicast,
        _marker: core::marker::PhantomData,
    })
}

fn decode_name(packet: &[u8], mut offset: usize, out: &mut String<64>) -> Option<usize> {
    let mut jumped = false;
    let mut jump_offset = 0usize;

    loop {
        if offset >= packet.len() {
            return None;
        }
        let len = packet[offset];
        if len & 0xC0 == 0xC0 {
            if offset + 1 >= packet.len() {
                return None;
            }
            let ptr = (((len & 0x3F) as usize) << 8) | packet[offset + 1] as usize;
            if !jumped {
                jump_offset = offset + 2;
                jumped = true;
            }
            offset = ptr;
            continue;
        } else if len == 0 {
            offset += 1;
            break;
        } else {
            offset += 1;
            if offset + len as usize > packet.len() {
                return None;
            }
            if !out.is_empty() {
                let _ = out.push('.');
            }
            for &b in &packet[offset..offset + len as usize] {
                let _ = out.push((b as char).to_ascii_lowercase());
            }
            offset += len as usize;
        }
    }

    Some(if jumped { jump_offset } else { offset })
}

fn name_matches(candidate: &str, target: &str) -> bool {
    if candidate.eq_ignore_ascii_case(target) {
        return true;
    }
    if let Some(stripped) = candidate.strip_suffix('.') {
        return stripped.eq_ignore_ascii_case(target);
    }
    false
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::device_identity::mac_to_string;

    #[test]
    fn hostname_and_fqdn_are_built_correctly() {
        let h = hostname_from_short_id("aabbcc");
        assert_eq!(h.as_str(), "isolarail-aabbcc");
        let fqdn = fqdn_from_hostname(h.as_str());
        assert_eq!(fqdn.as_str(), "isolarail-aabbcc.local");
        let inst = service_instance_name(h.as_str());
        assert_eq!(inst.as_str(), "isolarail-aabbcc._http._tcp.local");
    }

    #[test]
    fn encode_decode_roundtrip_for_a_response() {
        let cfg = MdnsConfig {
            hostname: String::from("isolarail-aabbcc"),
            hostname_fqdn: String::from("isolarail-aabbcc.local"),
            instance_name: String::from("isolarail-aabbcc._http._tcp.local"),
            port: 80,
        };
        let ip = Ipv4Address::new(192, 168, 1, 42);
        let mut buf = [0u8; 256];
        let len = build_a_only_response(&mut buf, &cfg, ip, None).unwrap();
        assert_eq!(&buf[0..2], &[0, 0]);
        assert_eq!(buf[2], 0x84);
        assert!(len > 12);
    }

    #[test]
    fn mac_string_helper_stays_available_for_network_identity() {
        assert_eq!(
            mac_to_string([0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]).as_str(),
            "02:aa:bb:cc:dd:ee"
        );
    }
}
