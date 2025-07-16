use defmt::*;
use w25::{W25, Q, Error};
use embedded_hal::digital::{OutputPin, PinState};

/// Dummy pin implementation for HOLD and WP pins
pub struct DummyPin;

impl OutputPin for DummyPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_state(&mut self, _state: PinState) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct DummyError;

impl defmt::Format for DummyError {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "DummyError");
    }
}

impl embedded_hal::digital::Error for DummyError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        embedded_hal::digital::ErrorKind::Other
    }
}

impl embedded_hal::digital::ErrorType for DummyPin {
    type Error = DummyError;
}

/// Flash programming operations
pub struct FlashProgrammer<SPI> {
    flash: W25<Q, SPI, DummyPin, DummyPin>,
}

impl<SPI> FlashProgrammer<SPI>
where
    SPI: embedded_hal_async::spi::SpiDevice,
    SPI::Error: core::fmt::Debug,
{
    /// Create a new Flash programmer instance
    pub fn new(flash: W25<Q, SPI, DummyPin, DummyPin>) -> Self {
        Self { flash }
    }

    /// Get device information
    pub async fn get_device_info(&mut self) -> Result<DeviceInfo, Error<SPI::Error, DummyError>> {
        let device_id_bytes = self.flash.device_id().await?;
        // Convert [u8; 8] to u32 by taking first 4 bytes
        let device_id = u32::from_be_bytes([device_id_bytes[0], device_id_bytes[1], device_id_bytes[2], device_id_bytes[3]]);

        Ok(DeviceInfo {
            device_id,
            status: 0, // w25 doesn't expose status register directly
            total_size: self.flash.capacity(),
            page_size: w25::Q::PAGE_SIZE,
            sector_size: w25::Q::SECTOR_SIZE,
            block_size: 65536, // 64K block size
        })
    }

    /// Erase a specific sector
    pub async fn erase_sector(&mut self, sector: u32) -> Result<(), Error<SPI::Error, DummyError>> {
        let n_sectors = self.flash.capacity() / w25::Q::SECTOR_SIZE;
        if sector >= n_sectors {
            return Err(Error::OutOfBounds);
        }

        info!("Erasing sector {} at address 0x{:06X}", sector, sector * w25::Q::SECTOR_SIZE);
        self.flash.erase_sector(sector).await
    }

    /// Erase a range of sectors
    pub async fn erase_sectors(&mut self, start_sector: u32, count: u32) -> Result<(), Error<SPI::Error, DummyError>> {
        for sector in start_sector..(start_sector + count) {
            self.erase_sector(sector).await?;
        }
        Ok(())
    }

    /// Erase entire chip
    pub async fn erase_chip(&mut self) -> Result<(), Error<SPI::Error, DummyError>> {
        info!("Erasing entire chip...");
        self.flash.erase_chip().await
    }

    /// Program data to Flash with automatic sector erase
    pub async fn program_data(&mut self, address: u32, data: &[u8]) -> Result<(), Error<SPI::Error, DummyError>> {
        info!("Programming {} bytes to Flash at address 0x{:06X}", data.len(), address);

        // Calculate sectors to erase
        let start_sector = address / w25q32jv::SECTOR_SIZE as u32;
        let end_address = address + data.len() as u32 - 1;
        let end_sector = end_address / w25q32jv::SECTOR_SIZE as u32;

        // Erase required sectors
        for sector in start_sector..=end_sector {
            self.erase_sector(sector).await?;
        }

        // Verify sector was erased
        let mut erase_verify = [0u8; 16];
        self.flash.read(address, &mut erase_verify).await?;
        info!("After sector erase, data at 0x{:06X}: {:?}", address, erase_verify);

        // Write data
        info!("Writing data...");
        info!("About to write {} bytes starting with: {:?}", data.len(), &data[..data.len().min(8)]);
        self.flash.write(address, data).await?;

        info!("Data written successfully");

        // Immediately read back to check what was written
        let verify_size = data.len().min(16);
        let mut immediate_verify = [0u8; 16];
        self.flash.read(address, &mut immediate_verify[..verify_size]).await?;
        info!("Immediate readback (first {} bytes): {:?}", verify_size, &immediate_verify[..verify_size]);
        Ok(())
    }

    /// Program data with verification
    pub async fn program_and_verify(&mut self, address: u32, data: &[u8]) -> Result<(), Error<SPI::Error, DummyError>> {
        // Program data
        self.program_data(address, data).await?;

        // Verify data
        info!("Verifying data...");
        self.verify_data(address, data).await?;

        info!("Programming and verification completed successfully");
        Ok(())
    }

    /// Verify data in Flash
    pub async fn verify_data(&mut self, address: u32, expected_data: &[u8]) -> Result<(), Error<SPI::Error, DummyError>> {
        const CHUNK_SIZE: usize = 1024; // 1KB chunks for verification
        let mut verified_bytes = 0;
        let mut current_address = address;

        while verified_bytes < expected_data.len() {
            let remaining = expected_data.len() - verified_bytes;
            let chunk_size = core::cmp::min(remaining, CHUNK_SIZE);

            let mut read_buffer = heapless::Vec::<u8, CHUNK_SIZE>::new();
            read_buffer.resize(chunk_size, 0).unwrap();

            self.flash.read(current_address, &mut read_buffer).await?;

            let expected_chunk = &expected_data[verified_bytes..verified_bytes + chunk_size];
            if read_buffer.as_slice() != expected_chunk {
                error!("Verification failed at address 0x{:06X}", current_address);
                return Err(Error::ReadbackFail); // Use readback fail for verification failure
            }

            verified_bytes += chunk_size;
            current_address += chunk_size as u32;
        }

        info!("Data verification completed successfully");
        Ok(())
    }

    /// Read data from Flash
    pub async fn read_data(&mut self, address: u32, buffer: &mut [u8]) -> Result<(), Error<SPI::Error, DummyError>> {
        self.flash.read(address, buffer).await
    }

    // Note: w25q32jv crate doesn't expose status register methods directly

    /// Dump Flash contents to defmt output
    pub async fn dump_flash(&mut self, start_address: u32, length: u32) -> Result<(), Error<SPI::Error, DummyError>> {
        const DUMP_CHUNK_SIZE: usize = 16; // 16 bytes per line
        let mut buffer = [0u8; DUMP_CHUNK_SIZE];
        let mut current_address = start_address;
        let end_address = start_address + length;

        info!("Flash dump from 0x{:06X} to 0x{:06X}:", start_address, end_address);

        while current_address < end_address {
            let remaining = (end_address - current_address) as usize;
            let chunk_size = core::cmp::min(remaining, DUMP_CHUNK_SIZE);

            self.flash.read(current_address, &mut buffer[..chunk_size]).await?;

            // Format output as hex dump
            info!("0x{:06X}: {:02X}", current_address, buffer[..chunk_size]);

            current_address += chunk_size as u32;
        }

        Ok(())
    }
}

/// Device information structure
#[derive(Debug)]
pub struct DeviceInfo {
    pub device_id: u32,
    pub status: u8,
    pub total_size: u32,
    pub page_size: u32,
    pub sector_size: u32,
    pub block_size: u32,
}

impl DeviceInfo {
    pub fn print_info(&self) {
        info!("=== W25Q128 Device Information ===");
        info!("Device ID: 0x{:06X}", self.device_id);
        info!("Status Register: 0x{:02X}", self.status);
        info!("Total Size: {} MB ({} bytes)", self.total_size / (1024 * 1024), self.total_size);
        info!("Page Size: {} bytes", self.page_size);
        info!("Sector Size: {} KB ({} bytes)", self.sector_size / 1024, self.sector_size);
        info!("Block Size: {} KB ({} bytes)", self.block_size / 1024, self.block_size);
        info!("Total Pages: {}", self.total_size / self.page_size);
        info!("Total Sectors: {}", self.total_size / self.sector_size);
        info!("Total Blocks: {}", self.total_size / self.block_size);
        info!("================================");
    }
}
