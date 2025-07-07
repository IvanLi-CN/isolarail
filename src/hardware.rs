// src/hardware.rs
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice as EmbassyI2cDevice;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as EmbassySpiDevice;
use embassy_stm32::gpio::{Level, Output, Speed, Input, Pull};
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::spi::{Config as SpiConfig, Spi as Stm32Spi};
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::time::{Hertz, khz};
use embassy_stm32::{bind_interrupts, mode, peripherals};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use gc9d01::{Config as DisplayDriverConfig, GC9D01, Orientation, Timer as Gc9d01Timer};
use static_cell::StaticCell;
use ina226::INA226;
use tca6424::{Tca6424, Pin, PinDirection, PinState};
use sw2303::SW2303;
use defmt::*;

// Interrupt bindings
bind_interrupts!(
    pub struct Irqs {
        I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
        I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    }
);

// Embassy display timer implementation
pub struct EmbassyDisplayTimer;
impl Gc9d01Timer for EmbassyDisplayTimer {
    async fn after_millis(milliseconds: u64) {
        embassy_time::Timer::after_millis(milliseconds).await;
    }
}

impl embedded_hal::digital::ErrorType for EmbassyDisplayTimer {
    type Error = core::convert::Infallible;
}

/// Five-way joystick GPIO configuration
pub struct FiveWayJoystick {
    pub up: Input<'static>,      // PA1
    pub down: Input<'static>,    // PA3
    pub left: Input<'static>,    // PA2
    pub right: Input<'static>,   // PA5
    pub center: Input<'static>,  // PA6
}

impl FiveWayJoystick {
    /// Check if UP button is pressed (active low)
    pub fn is_up_pressed(&self) -> bool {
        self.up.is_low()
    }

    /// Check if DOWN button is pressed (active low)
    pub fn is_down_pressed(&self) -> bool {
        self.down.is_low()
    }

    /// Check if LEFT button is pressed (active low)
    pub fn is_left_pressed(&self) -> bool {
        self.left.is_low()
    }

    /// Check if RIGHT button is pressed (active low)
    pub fn is_right_pressed(&self) -> bool {
        self.right.is_low()
    }

    /// Check if CENTER button is pressed (active low)
    pub fn is_center_pressed(&self) -> bool {
        self.center.is_low()
    }

    /// Get all button states as a tuple (up, down, left, right, center)
    pub fn get_all_states(&self) -> (bool, bool, bool, bool, bool) {
        (
            self.is_up_pressed(),
            self.is_down_pressed(),
            self.is_left_pressed(),
            self.is_right_pressed(),
            self.is_center_pressed(),
        )
    }
}

// Hardware configuration structure
pub struct HardwareConfig<'a> {
    pub ina226_sensors: (
        INA226<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>>,
        INA226<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>>,
        INA226<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>>,
    ),
    pub tca6424_expander: Tca6424<'a, EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>>,
    pub sw2303_controller: SW2303<'a, EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>>,
    pub buzzer_pwm: SimplePwm<'static, peripherals::TIM3>,
    pub backlight_pwm: SimplePwm<'static, peripherals::TIM1>,
    pub joystick: FiveWayJoystick,
    pub display: GC9D01<
        'static,
        EmbassySpiDevice<
            'static,
            CriticalSectionRawMutex,
            Stm32Spi<'static, mode::Async>,
            Output<'static>,
        >,
        Output<'static>,
        Output<'static>,
        EmbassyDisplayTimer,
    >,
}

/// Configure STM32 system
pub fn configure_stm32() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi48 = Some(Hsi48Config {
            sync_from_usb: true,
        });
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: None,
            divq: None,
            // Main system clock at 170 MHz
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
    }
    config
}

/// Configure SW2303 for 65W power delivery using REG 0xAF power configuration.
/// This is the business logic for our specific application requirements.
pub async fn configure_sw2303_power<I2C>(sw2303: &mut SW2303<'_, I2C>) -> Result<(), sw2303::error::Error<I2C::Error>>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    info!("Configuring SW2303 for 65W power with 100mA detection threshold");

    // Power configuration (REG 0xAF) requires unlock_write_enable_0()
    // as it's in the locked register range (0xA0-0xBF)
    sw2303.unlock_write_enable_0().await?;
    info!("SW2303 register unlock sequence completed for power configuration");

    // Configure power for 65W using REG 0xAF (Power Configuration register)
    sw2303.set_power_config(65).await?;
    info!("SW2303 power configured to 65W using REG 0xAF");

    // Verify configuration by reading back the power setting
    let (register_mode, power_watts) = sw2303.get_power_config().await?;
    info!("SW2303 power configuration verification - Register mode: {}, Power: {}W", register_mode, power_watts);

    info!("SW2303 power configuration completed: 65W using REG 0xAF");
    Ok(())
}

/// Apply dynamic power allocation to hardware controllers.
/// This function configures SW2303 and TPS25810 controllers based on calculated power allocation.
pub async fn apply_power_allocation<I2C>(
    sw2303: &mut SW2303<'_, I2C>,
    tca6424: &mut Tca6424<'_, I2C>,
    power_allocation: [f32; 3]
) -> Result<(), sw2303::error::Error<I2C::Error>>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    info!("Applying power allocation: P1={}W, P2={}W, P3={}W",
          power_allocation[0], power_allocation[1], power_allocation[2]);

    // Apply Port 1 (SW2303) power allocation
    let port1_power = power_allocation[0] as u8;
    if port1_power > 0 && port1_power <= 127 {
        info!("Configuring SW2303 Port 1 to {}W", port1_power);
        match sw2303.unlock_write_enable_0().await {
            Ok(_) => {
                info!("SW2303 registers unlocked successfully");
                match sw2303.set_power_config(port1_power).await {
                    Ok(_) => {
                        info!("✓ SW2303 Port 1 power configured to {}W", port1_power);
                    }
                    Err(e) => {
                        info!("✗ Failed to set SW2303 power config");
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                info!("✗ Failed to unlock SW2303 registers");
                return Err(e);
            }
        }
    } else if port1_power == 0 {
        info!("Port 1 power allocation is 0W, skipping SW2303 configuration");
    } else {
        info!("Port 1 power allocation {}W exceeds maximum 127W, skipping", port1_power);
    }

    // Apply Port 2 (TPS25810) current allocation
    let port2_current_a = power_allocation[1] / 5.0; // Convert watts to amperes (5V)
    info!("Configuring TPS25810 Port 2 to {}A ({}W)", port2_current_a as u32, power_allocation[1] as u32);
    match apply_tps25810_current_limit(tca6424, 2, port2_current_a).await {
        Ok(_) => {
            info!("✓ Port 2 current limit configured successfully");
        }
        Err(_e) => {
            info!("✗ Failed to set Port 2 current limit");
        }
    }

    // Apply Port 3 (TPS25810) current allocation
    let port3_current_a = power_allocation[2] / 5.0; // Convert watts to amperes (5V)
    info!("Configuring TPS25810 Port 3 to {}A ({}W)", port3_current_a as u32, power_allocation[2] as u32);
    match apply_tps25810_current_limit(tca6424, 3, port3_current_a).await {
        Ok(_) => {
            info!("✓ Port 3 current limit configured successfully");
        }
        Err(_e) => {
            info!("✗ Failed to set Port 3 current limit");
        }
    }

    info!("✓ Hardware configured: P1={}W, P2={}A, P3={}A",
          port1_power, port2_current_a as u32, port3_current_a as u32);

    Ok(())
}

/// Apply current limit to TPS25810 controller via TCA6424 GPIO pins.
///
/// # Arguments
/// * `tca6424` - TCA6424 I/O expander reference
/// * `port` - Port number (2 or 3)
/// * `current_a` - Current limit in amperes
async fn apply_tps25810_current_limit<I2C>(
    tca6424: &mut Tca6424<'_, I2C>,
    port: u8,
    current_a: f32
) -> Result<(), tca6424::errors::Error<I2C::Error>>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    use tca6424::{Pin, PinState};

    info!("Setting TPS25810 Port {} current limit to {}A", port, current_a as u32);

    let (chg_pin, chg_hl_pin) = match port {
        2 => {
            info!("Port 2 GPIO pins: CHG=P04, CHG_HL=P03");
            (Pin::P04, Pin::P03) // Port 2: P2_CHG, P2_CHG_HL
        },
        3 => {
            info!("Port 3 GPIO pins: CHG=P22, CHG_HL=P23");
            (Pin::P22, Pin::P23) // Port 3: P3_CHG, P3_CHG_HL
        },
        _ => {
            info!("Invalid port number: {}", port);
            return Err(tca6424::errors::Error::InvalidRegisterOrPin);
        }
    };

    if current_a >= 2.5 {
        // 3A mode: CHG = High, CHG_HL = High
        tca6424.set_pin_output(chg_pin, PinState::High).await?;
        tca6424.set_pin_output(chg_hl_pin, PinState::High).await?;
        info!("✓ Port {} configured for 3A", port);
    } else if current_a >= 1.0 {
        // 1.5A mode: CHG = High, CHG_HL = Low
        tca6424.set_pin_output(chg_pin, PinState::High).await?;
        tca6424.set_pin_output(chg_hl_pin, PinState::Low).await?;
        info!("✓ Port {} configured for 1.5A", port);
    } else {
        // Low current mode: CHG = Low, CHG_HL = Low (500mA/900mA default)
        tca6424.set_pin_output(chg_pin, PinState::Low).await?;
        tca6424.set_pin_output(chg_hl_pin, PinState::Low).await?;
        info!("✓ Port {} configured for default current", port);
    }

    Ok(())
}

/// Control USB communication for a specific port
///
/// # Arguments
/// * `tca6424` - TCA6424 expander instance
/// * `port` - Port number (1, 2, or 3)
/// * `enable` - true to enable communication, false to disable
pub async fn control_usb_communication<I2C>(
    tca6424: &mut Tca6424<'_, I2C>,
    port: u8,
    enable: bool
) -> Result<(), tca6424::errors::Error<I2C::Error>>
where
    I2C: embedded_hal_async::i2c::I2c,
{
    let data_conn_pin = match port {
        1 => Pin::P10, // P1_DATA_CONN
        2 => Pin::P11, // P2_DATA_CONN
        3 => Pin::P12, // P3_DATA_CONN
        _ => {
            info!("Invalid port number: {}", port);
            return Err(tca6424::errors::Error::InvalidRegisterOrPin);
        }
    };

    let state = if enable { PinState::High } else { PinState::Low };
    tca6424.set_pin_output(data_conn_pin, state).await?;

    let action = if enable { "enabled" } else { "disabled" };
    info!("✓ Port {} USB communication {}", port, action);

    Ok(())
}

/// Initialize all hardware components
pub async fn initialize_hardware(p: embassy_stm32::Peripherals) -> HardwareConfig<'static> {
    info!("Initializing hardware components...");

    // Initialize I2C1
    let i2c_scl = p.PA15; // SCL pin for I2C1
    let i2c_sda = p.PB7; // SDA pin for I2C1

    let mut i2c_config = i2c::Config::default();

    // Initialize I2C1 with correct parameter order
    let i2c1 = I2c::new(
        p.I2C1,     // Instance
        i2c_scl,    // SCL pin
        i2c_sda,    // SDA pin
        Irqs,       // Interrupts struct
        p.DMA1_CH2, // RX DMA
        p.DMA1_CH3, // TX DMA
        khz(100),   // Frequency
        i2c_config, // Config parameter
    );

    // Create a static mutex for the I2C bus
    static I2C1_BUS_CELL: StaticCell<Mutex<CriticalSectionRawMutex, I2c<'static, mode::Async>>> =
        StaticCell::new();
    let i2c1_bus_mutex_ref = I2C1_BUS_CELL.init(Mutex::new(i2c1));

    // Initialize INA226 sensors
    let i2c_device_1 = EmbassyI2cDevice::new(i2c1_bus_mutex_ref);
    let i2c_device_2 = EmbassyI2cDevice::new(i2c1_bus_mutex_ref);
    let i2c_device_3 = EmbassyI2cDevice::new(i2c1_bus_mutex_ref);

    let mut ina226_1 = INA226::new(i2c_device_1, 0x40);
    let mut ina226_2 = INA226::new(i2c_device_2, 0x41);
    let mut ina226_3 = INA226::new(i2c_device_3, 0x44);

    // Configure INA226 sensors
    ina226_1.callibrate(0.005, 4.0).await.unwrap();
    ina226_2.callibrate(0.010, 4.0).await.unwrap();
    ina226_3.callibrate(0.010, 4.0).await.unwrap();
    info!("INA226 sensors initialized.");

    // Initialize TCA6424
    static I2C_DEVICE_TCA6424_CELL: StaticCell<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>> = StaticCell::new();
    let mut i2c_device_tca6424 = I2C_DEVICE_TCA6424_CELL.init(EmbassyI2cDevice::new(i2c1_bus_mutex_ref));
    let mut tca6424_expander = Tca6424::new(i2c_device_tca6424, tca6424::DEFAULT_ADDRESS).unwrap();
    
    // Configure input pins for UFP detection and fault monitoring
    tca6424_expander.set_pin_direction(Pin::P01, PinDirection::Input).await.unwrap(); // P2_UFP
    tca6424_expander.set_pin_direction(Pin::P25, PinDirection::Input).await.unwrap(); // P3_UFP
    tca6424_expander.set_pin_direction(Pin::P06, PinDirection::Input).await.unwrap(); // P2_FAULT
    tca6424_expander.set_pin_direction(Pin::P20, PinDirection::Input).await.unwrap(); // P3_FAULT

    // Configure output pins for current control
    // Port 2 current control pins
    tca6424_expander.set_pin_direction(Pin::P04, PinDirection::Output).await.unwrap(); // P2_CHG
    tca6424_expander.set_pin_direction(Pin::P03, PinDirection::Output).await.unwrap(); // P2_CHG_HL

    // Port 3 current control pins
    tca6424_expander.set_pin_direction(Pin::P22, PinDirection::Output).await.unwrap(); // P3_CHG
    tca6424_expander.set_pin_direction(Pin::P23, PinDirection::Output).await.unwrap(); // P3_CHG_HL

    // Configure output pins for USB communication control
    tca6424_expander.set_pin_direction(Pin::P10, PinDirection::Output).await.unwrap(); // P1_DATA_CONN
    tca6424_expander.set_pin_direction(Pin::P11, PinDirection::Output).await.unwrap(); // P2_DATA_CONN
    tca6424_expander.set_pin_direction(Pin::P12, PinDirection::Output).await.unwrap(); // P3_DATA_CONN

    // Enable 3A current capability for Port 2
    // First enable P2_CHG (1.5A base current source)
    tca6424_expander.set_pin_output(Pin::P04, PinState::High).await.unwrap(); // P2_CHG = High
    // Then enable P2_CHG_HL (3A current source, requires P2_CHG to be High)
    tca6424_expander.set_pin_output(Pin::P03, PinState::High).await.unwrap(); // P2_CHG_HL = High

    // Enable 3A current capability for Port 3
    // First enable P3_CHG (1.5A base current source)
    tca6424_expander.set_pin_output(Pin::P22, PinState::High).await.unwrap(); // P3_CHG = High
    // Then enable P3_CHG_HL (3A current source, requires P3_CHG to be High)
    tca6424_expander.set_pin_output(Pin::P23, PinState::High).await.unwrap(); // P3_CHG_HL = High

    // Enable USB communication for all ports by default
    tca6424_expander.set_pin_output(Pin::P10, PinState::High).await.unwrap(); // P1_DATA_CONN = High (enabled)
    tca6424_expander.set_pin_output(Pin::P11, PinState::High).await.unwrap(); // P2_DATA_CONN = High (enabled)
    tca6424_expander.set_pin_output(Pin::P12, PinState::High).await.unwrap(); // P3_DATA_CONN = High (enabled)

    info!("TCA6424 expander initialized with 3A current capability and USB communication enabled for all ports.");

    // Initialize SW2303
    static I2C_DEVICE_SW2303_CELL: StaticCell<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>> = StaticCell::new();
    let mut i2c_device_sw2303 = I2C_DEVICE_SW2303_CELL.init(EmbassyI2cDevice::new(i2c1_bus_mutex_ref));
    let mut sw2303_controller = SW2303::new(i2c_device_sw2303, sw2303::registers::constants::DEFAULT_ADDRESS);

    // Initialize SW2303 PD controller
    match sw2303_controller.init().await {
        Ok(_) => {
            info!("SW2303 PD controller initialized successfully.");
            
            // Configure SW2303 for 65W power
            match configure_sw2303_power(&mut sw2303_controller).await {
                Ok(_) => info!("SW2303 configured for 65W power with 100mA detection threshold."),
                Err(e) => {
                    error!("Failed to configure SW2303 power settings: {:?}", e);
                }
            }
        },
        Err(e) => {
            error!("Failed to initialize SW2303: {:?}", e);
        }
    }

    // Initialize buzzer PWM
    let buzzer_pin = PwmPin::new_ch1(p.PC6, embassy_stm32::gpio::OutputType::PushPull);
    let mut buzzer_pwm = SimplePwm::new(
        p.TIM3,
        Some(buzzer_pin),
        None,
        None,
        None,
        Hertz(2000),
        CountingMode::EdgeAlignedUp,
    );
    buzzer_pwm.ch1().set_duty_cycle_percent(0);
    info!("Buzzer PWM initialized on PC6 (TIM3_CH1).");

    // Initialize backlight PWM
    let backlight_pin = PwmPin::new_ch1(p.PA8, embassy_stm32::gpio::OutputType::PushPull);
    let mut backlight_pwm = SimplePwm::new(
        p.TIM1,
        Some(backlight_pin),
        None,
        None,
        None,
        Hertz(1000), // 1kHz PWM frequency for backlight
        CountingMode::EdgeAlignedUp,
    );
    // Set backlight to 75% brightness
    backlight_pwm.ch1().set_duty_cycle_percent(75);
    backlight_pwm.ch1().enable();
    info!("Backlight PWM initialized on PA8 (TIM1_CH1) with 75% brightness.");

    // Initialize SPI and display
    let spi_peripheral_instance = p.SPI1;
    let sck_pin = p.PB3;
    let mosi_pin = p.PA7;
    let cs_pin_output = Output::new(p.PA4, Level::High, Speed::VeryHigh);
    let dc_pin = Output::new(p.PB0, Level::Low, Speed::VeryHigh);
    let rst_pin = Output::new(p.PC4, Level::Low, Speed::VeryHigh);

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(48_000_000);

    let spi_bus = Stm32Spi::new_txonly(
        spi_peripheral_instance,
        sck_pin,
        mosi_pin,
        p.DMA1_CH1,
        spi_config,
    );

    static SPI_BUS_CELL: StaticCell<
        Mutex<CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>>,
    > = StaticCell::new();
    let spi_bus_mutex_ref = SPI_BUS_CELL.init(Mutex::new(spi_bus));

    let spi_device = EmbassySpiDevice::<
        'static,
        CriticalSectionRawMutex,
        Stm32Spi<'static, mode::Async>,
        Output<'static>,
    >::new(spi_bus_mutex_ref, cs_pin_output);

    let display_config = DisplayDriverConfig {
        width: 160,
        height: 40,
        orientation: Orientation::PortraitSwapped,
        rgb: false,
        inverted: false,
        dx: 0,
        dy: 0,
    };

    // Initialize five-way joystick
    info!("Initializing five-way joystick...");
    let joystick = FiveWayJoystick {
        up: Input::new(p.PA1, Pull::Up),       // UP button on PA1
        down: Input::new(p.PA3, Pull::Up),     // DOWN button on PA3
        left: Input::new(p.PA2, Pull::Up),     // LEFT button on PA2
        right: Input::new(p.PA5, Pull::Up),    // RIGHT button on PA5
        center: Input::new(p.PA6, Pull::Up),   // CENTER button on PA6
    };
    info!("Five-way joystick initialized on PA1(UP), PA3(DOWN), PA2(LEFT), PA5(RIGHT), PA6(CENTER).");

    static DISPLAY_BUFFER_CELL: StaticCell<[u8; gc9d01::BUF_SIZE]> = StaticCell::new();
    let buffer_slice: &mut [u8] = DISPLAY_BUFFER_CELL.init([0; gc9d01::BUF_SIZE]);

    let mut display: GC9D01<
        '_,
        EmbassySpiDevice<
            'static,
            CriticalSectionRawMutex,
            Stm32Spi<'static, mode::Async>,
            Output<'static>,
        >,
        Output<'_>,
        Output<'_>,
        EmbassyDisplayTimer,
    > = GC9D01::new(display_config, spi_device, dc_pin, rst_pin, buffer_slice);

    info!("Initializing display...");
    match display.init().await {
        Ok(_) => info!("Display initialized successfully!"),
        Err(e) => error!("Display initialization failed: {:?}", e),
    }

    info!("Hardware initialization complete.");

    HardwareConfig {
        ina226_sensors: (ina226_1, ina226_2, ina226_3),
        tca6424_expander,
        sw2303_controller,
        buzzer_pwm,
        backlight_pwm,
        joystick,
        display,
    }
}
