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
use tca6424::{Tca6424, Pin, PinDirection};
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
    
    // Configure P01 (Port 2 UFP) and P25 (Port 3 UFP) as inputs
    tca6424_expander.set_pin_direction(Pin::P01, PinDirection::Input).await.unwrap();
    tca6424_expander.set_pin_direction(Pin::P25, PinDirection::Input).await.unwrap();
    info!("TCA6424 expander initialized.");

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
        joystick,
        display,
    }
}
