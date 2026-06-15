use core::str;

use embassy_time::{Duration, Timer};
use embedded_hal::i2c::{Error, ErrorKind, SevenBitAddress};
use embedded_hal_async::i2c::{I2c, Operation};

pub const WIFI_EEPROM_ADDR_7BIT: SevenBitAddress = 0x50;

const RECORD_LEN: usize = 160;
const MAGIC: &[u8; 8] = b"IPWIFI1\0";
const VERSION: u8 = 2;
const EEPROM_RECORD_OFFSET: u16 = 0;
const EEPROM_PAGE_SIZE: usize = 16;
const EEPROM_WRITE_CYCLE: Duration = Duration::from_millis(6);
const FLAG_STATIC_IPV4: u8 = 1 << 0;
const FLAG_STATIC_DNS: u8 = 1 << 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticIpv4Config {
    pub address: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns: Option<[u8; 4]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiCredentials {
    ssid: [u8; 32],
    ssid_len: u8,
    psk: [u8; 64],
    psk_len: u8,
    static_ipv4: Option<StaticIpv4Config>,
}

impl WifiCredentials {
    pub fn new(ssid: &str, psk: &str) -> Result<Self, ProvisioningError<()>> {
        let ssid_b = ssid.as_bytes();
        let psk_b = psk.as_bytes();
        if ssid_b.is_empty()
            || ssid_b.len() > 32
            || (!psk_b.is_empty() && psk_b.len() < 8)
            || psk_b.len() > 64
        {
            return Err(ProvisioningError::InvalidInput);
        }

        let mut out = Self {
            ssid: [0; 32],
            ssid_len: ssid_b.len() as u8,
            psk: [0; 64],
            psk_len: psk_b.len() as u8,
            static_ipv4: None,
        };
        out.ssid[..ssid_b.len()].copy_from_slice(ssid_b);
        out.psk[..psk_b.len()].copy_from_slice(psk_b);
        Ok(out)
    }

    #[allow(dead_code)]
    pub fn ssid(&self) -> &str {
        str::from_utf8(&self.ssid[..self.ssid_len as usize]).unwrap_or("")
    }

    #[allow(dead_code)]
    pub fn psk(&self) -> &str {
        str::from_utf8(&self.psk[..self.psk_len as usize]).unwrap_or("")
    }

    pub const fn psk_configured(&self) -> bool {
        self.psk_len > 0
    }

    pub const fn static_ipv4(&self) -> Option<StaticIpv4Config> {
        self.static_ipv4
    }

    #[allow(dead_code)]
    pub fn with_static_ipv4(mut self, static_ipv4: StaticIpv4Config) -> Self {
        self.static_ipv4 = Some(static_ipv4);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisioningError<E> {
    Bus(E),
    InvalidInput,
    InvalidRecord,
}

impl<E: Error> Error for ProvisioningError<E> {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Bus(e) => e.kind(),
            Self::InvalidInput | Self::InvalidRecord => ErrorKind::Other,
        }
    }
}

pub async fn load_wifi_credentials<I2C>(
    i2c: &mut I2C,
) -> Result<Option<WifiCredentials>, ProvisioningError<I2C::Error>>
where
    I2C: I2c<SevenBitAddress>,
{
    let mut record = [0u8; RECORD_LEN];
    eeprom_read(i2c, EEPROM_RECORD_OFFSET, &mut record).await?;

    if record.iter().all(|b| *b == 0x00 || *b == 0xff) {
        return Ok(None);
    }
    if &record[..MAGIC.len()] != MAGIC || record[MAGIC.len()] != VERSION {
        return Err(ProvisioningError::InvalidRecord);
    }

    let ssid_len = record[12] as usize;
    let psk_len = record[13] as usize;
    if ssid_len == 0 || ssid_len > 32 || psk_len > 64 {
        return Err(ProvisioningError::InvalidRecord);
    }
    let flags = record[9];

    let checksum_offset = RECORD_LEN - 4;
    let expected = u32::from_le_bytes([
        record[checksum_offset],
        record[checksum_offset + 1],
        record[checksum_offset + 2],
        record[checksum_offset + 3],
    ]);
    record[checksum_offset..].fill(0);
    if checksum(&record) != expected {
        return Err(ProvisioningError::InvalidRecord);
    }

    let mut ssid = [0u8; 32];
    let mut psk = [0u8; 64];
    ssid[..ssid_len].copy_from_slice(&record[16..16 + ssid_len]);
    psk[..psk_len].copy_from_slice(&record[48..48 + psk_len]);
    let static_ipv4 = if flags & FLAG_STATIC_IPV4 != 0 {
        let address = [record[112], record[113], record[114], record[115]];
        let netmask = [record[116], record[117], record[118], record[119]];
        let gateway = [record[120], record[121], record[122], record[123]];
        let dns = if flags & FLAG_STATIC_DNS != 0 {
            Some([record[124], record[125], record[126], record[127]])
        } else {
            None
        };
        Some(StaticIpv4Config {
            address,
            netmask,
            gateway,
            dns,
        })
    } else {
        None
    };
    Ok(Some(WifiCredentials {
        ssid,
        ssid_len: ssid_len as u8,
        psk,
        psk_len: psk_len as u8,
        static_ipv4,
    }))
}

pub async fn store_wifi_credentials<I2C>(
    i2c: &mut I2C,
    credentials: &WifiCredentials,
) -> Result<(), ProvisioningError<I2C::Error>>
where
    I2C: I2c<SevenBitAddress>,
{
    let mut record = [0u8; RECORD_LEN];
    record[..MAGIC.len()].copy_from_slice(MAGIC);
    record[MAGIC.len()] = VERSION;
    if let Some(static_ipv4) = credentials.static_ipv4 {
        record[9] |= FLAG_STATIC_IPV4;
        record[112..116].copy_from_slice(&static_ipv4.address);
        record[116..120].copy_from_slice(&static_ipv4.netmask);
        record[120..124].copy_from_slice(&static_ipv4.gateway);
        if let Some(dns) = static_ipv4.dns {
            record[9] |= FLAG_STATIC_DNS;
            record[124..128].copy_from_slice(&dns);
        }
    }
    record[12] = credentials.ssid_len;
    record[13] = credentials.psk_len;
    record[16..16 + credentials.ssid_len as usize]
        .copy_from_slice(&credentials.ssid[..credentials.ssid_len as usize]);
    record[48..48 + credentials.psk_len as usize]
        .copy_from_slice(&credentials.psk[..credentials.psk_len as usize]);

    let crc = checksum(&record);
    record[RECORD_LEN - 4..].copy_from_slice(&crc.to_le_bytes());
    eeprom_write(i2c, EEPROM_RECORD_OFFSET, &record).await
}

pub async fn clear_wifi_credentials<I2C>(i2c: &mut I2C) -> Result<(), ProvisioningError<I2C::Error>>
where
    I2C: I2c<SevenBitAddress>,
{
    eeprom_write(i2c, EEPROM_RECORD_OFFSET, &[0u8; RECORD_LEN]).await
}

async fn eeprom_read<I2C>(
    i2c: &mut I2C,
    offset: u16,
    buf: &mut [u8],
) -> Result<(), ProvisioningError<I2C::Error>>
where
    I2C: I2c<SevenBitAddress>,
{
    let addr = offset.to_be_bytes();
    i2c.transaction(
        WIFI_EEPROM_ADDR_7BIT,
        &mut [Operation::Write(&addr), Operation::Read(buf)],
    )
    .await
    .map_err(ProvisioningError::Bus)
}

async fn eeprom_write<I2C>(
    i2c: &mut I2C,
    mut offset: u16,
    mut bytes: &[u8],
) -> Result<(), ProvisioningError<I2C::Error>>
where
    I2C: I2c<SevenBitAddress>,
{
    let mut page = [0u8; EEPROM_PAGE_SIZE + 2];
    while !bytes.is_empty() {
        let page_room = EEPROM_PAGE_SIZE - (offset as usize % EEPROM_PAGE_SIZE);
        let n = bytes.len().min(page_room);
        page[..2].copy_from_slice(&offset.to_be_bytes());
        page[2..2 + n].copy_from_slice(&bytes[..n]);
        i2c.write(WIFI_EEPROM_ADDR_7BIT, &page[..2 + n])
            .await
            .map_err(ProvisioningError::Bus)?;
        Timer::after(EEPROM_WRITE_CYCLE).await;
        offset += n as u16;
        bytes = &bytes[n..];
    }
    Ok(())
}

fn checksum(bytes: &[u8]) -> u32 {
    let mut h = 0x811c_9dc5u32;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}
