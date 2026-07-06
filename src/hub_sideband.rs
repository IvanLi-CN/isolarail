use embedded_hal_async::i2c::I2c;

pub const TCA6408_ADDR: u8 = 0x20;

const REG_INPUT: u8 = 0x00;
const REG_OUTPUT: u8 = 0x01;
const REG_POLARITY: u8 = 0x02;
const REG_CONFIG: u8 = 0x03;
const PWREN_MASK: u8 = 0b0101_0101;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub input: u8,
    pub output: u8,
    pub polarity: u8,
    pub config: u8,
    pub pwren_enabled: [bool; 4],
    pub ovcur_asserted: [bool; 4],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Controller {
    output: u8,
    config: u8,
}

impl Controller {
    pub async fn init<I2C: I2c>(i2c: &mut I2C) -> Result<Self, I2C::Error> {
        let ctrl = Self {
            output: 0xFF,
            config: 0xFF,
        };
        ctrl.write_output(i2c).await?;
        i2c.write(TCA6408_ADDR, &[REG_POLARITY, 0x00]).await?;
        ctrl.write_config(i2c).await?;
        Ok(ctrl)
    }

    pub async fn snapshot<I2C: I2c>(&self, i2c: &mut I2C) -> Result<Snapshot, I2C::Error> {
        let input = read_reg(i2c, REG_INPUT).await?;
        let output = read_reg(i2c, REG_OUTPUT).await?;
        let polarity = read_reg(i2c, REG_POLARITY).await?;
        let config = read_reg(i2c, REG_CONFIG).await?;
        let mut pwren_enabled = [false; 4];
        let mut ovcur_asserted = [false; 4];
        for ch in 0..4 {
            let pwren = pwren_bit(ch);
            let ovcur = ovcur_bit(ch);
            pwren_enabled[ch as usize] = (input & pwren) == 0;
            ovcur_asserted[ch as usize] = (config & ovcur) == 0 && (output & ovcur) == 0;
        }
        Ok(Snapshot {
            input,
            output,
            polarity,
            config,
            pwren_enabled,
            ovcur_asserted,
        })
    }

    pub async fn set_overcurrent<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        ch: u8,
        asserted: bool,
    ) -> Result<(), I2C::Error> {
        let bit = ovcur_bit(ch);
        let next_output;
        let next_config;
        if asserted {
            next_output = self.output & !bit;
            next_config = (self.config & !bit) | PWREN_MASK;
        } else {
            next_output = self.output | bit;
            next_config = self.config | bit | PWREN_MASK;
        }
        if self.output == next_output && self.config == next_config {
            return Ok(());
        }
        if self.output != next_output {
            self.output = next_output;
            self.write_output(i2c).await?;
        }
        if self.config != next_config {
            self.config = next_config;
            self.write_config(i2c).await?;
        }
        Ok(())
    }

    async fn write_output<I2C: I2c>(&self, i2c: &mut I2C) -> Result<(), I2C::Error> {
        i2c.write(TCA6408_ADDR, &[REG_OUTPUT, self.output]).await
    }

    async fn write_config<I2C: I2c>(&self, i2c: &mut I2C) -> Result<(), I2C::Error> {
        i2c.write(TCA6408_ADDR, &[REG_CONFIG, self.config]).await
    }
}

async fn read_reg<I2C: I2c>(i2c: &mut I2C, reg: u8) -> Result<u8, I2C::Error> {
    let mut b = [0u8; 1];
    i2c.write_read(TCA6408_ADDR, &[reg], &mut b).await?;
    Ok(b[0])
}

#[inline]
fn pwren_bit(ch: u8) -> u8 {
    1u8 << ((ch & 0x03) * 2)
}

#[inline]
fn ovcur_bit(ch: u8) -> u8 {
    1u8 << (((ch & 0x03) * 2) + 1)
}
