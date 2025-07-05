// src/main.rs
#![no_std]
#![no_main]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as EmbassySpiDevice;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::i2c::{self, I2c}; // Import i2c module, I2c struct
use embassy_stm32::spi::{Config as SpiConfig, Spi as Stm32Spi};
use embassy_stm32::timer::{simple_pwm::{PwmPin, SimplePwm}, low_level::CountingMode};
use embassy_stm32::time::{Hertz, khz}; // Import khz and Hertz
use embassy_stm32::{bind_interrupts, mode, peripherals}; // Import bind_interrupts, mode, peripherals
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_alloc::LlffHeap as Heap;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::WebColors;
use gc9d01::{Config as DisplayDriverConfig, GC9D01, Orientation, Timer as Gc9d01Timer};
use static_cell::StaticCell;

use core::ptr;
use {defmt_rtt as _, panic_probe as _};

// Add imports for INA226 and shared bus I2C device
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice as EmbassyI2cDevice; // Alias for clarity
use ina226::INA226;
use tca6424::{Tca6424, Pin, PinDirection, PinState};
use sw2303::SW2303;
// Removed unused imports: AsyncI2c

use defmt::*;
use display::dashboard::Dashboard;
mod display;

extern crate alloc;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// This marks the entrypoint of our application and binds interrupts.
bind_interrupts!(
    struct Irqs {
        I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
        I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    }
);

/// Configure SW2303 for 65W power delivery using REG 0xAF power configuration.
/// This is the business logic for our specific application requirements.
async fn configure_sw2303_power<I2C>(sw2303: &mut SW2303<'_, I2C>) -> Result<(), sw2303::error::Error<I2C::Error>>
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

/// Buzzer control function to emit a beep sound
async fn beep_buzzer(buzzer_pwm: &mut SimplePwm<'_, peripherals::TIM3>, duration_ms: u64) {
    // Enable PWM channel and set 80% duty cycle for louder beep
    buzzer_pwm.ch1().enable();
    buzzer_pwm.ch1().set_duty_cycle_percent(80);

    // Wait for the specified duration
    embassy_time::Timer::after_millis(duration_ms).await;

    // Turn off the buzzer
    buzzer_pwm.ch1().set_duty_cycle_percent(0);
    buzzer_pwm.ch1().disable();
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting GC9D01 Example");

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
        // config.enable_ucpd1_dead_battery = true;
    }
    let p = embassy_stm32::init(config);

    // Initialize the allocator BEFORE you use it
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 8192;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    // Initialize I2C1
    let i2c_scl = p.PA15; // SCL pin for I2C1
    let i2c_sda = p.PB7; // SDA pin for I2C1

    let mut i2c_config = i2c::Config::default(); // Use full path for Config
    // Configure I2C speed if needed, default is 100kHz
    // i2c_config.speed = embassy_stm32::i2c::Speed::Standard; // Or Fast, FastPlus

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

    // Create a static mutex for the I2C bus using the full I2c type
    // Create a static mutex for the I2C bus using the full I2c type
    static I2C1_BUS_CELL: StaticCell<Mutex<CriticalSectionRawMutex, I2c<'static, mode::Async>>> =
        StaticCell::new(); // Use full I2c type
    let i2c1_bus_mutex_ref = I2C1_BUS_CELL.init(Mutex::new(i2c1));

    // Initialize INA226 sensors using I2cDevice for shared bus access
    // Create I2cDevice instances from the shared bus mutex with correct type parameters
    let i2c_device_1 = EmbassyI2cDevice::new(i2c1_bus_mutex_ref);
    let i2c_device_2 = EmbassyI2cDevice::new(i2c1_bus_mutex_ref);
    let i2c_device_3 = EmbassyI2cDevice::new(i2c1_bus_mutex_ref);

    // Initialize INA226 sensors with the I2cDevice instances
    let mut ina226_1 = INA226::new(i2c_device_1, 0x40);
    let mut ina226_2 = INA226::new(i2c_device_2, 0x41);
    let mut ina226_3 = INA226::new(i2c_device_3, 0x44);

    // Configure INA226 sensors (optional, default config is often fine)
    // Example: Set calibration register for current/power readings
    ina226_1.callibrate(0.005, 4.0).await.unwrap();
    ina226_2.callibrate(0.010, 4.0).await.unwrap();
    ina226_3.callibrate(0.010, 4.0).await.unwrap();

    info!("INA226 sensors initialized.");

    // Create I2cDevice instance for TCA6424
    static I2C_DEVICE_TCA6424_CELL: StaticCell<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>> = StaticCell::new();
    let mut i2c_device_tca6424 = I2C_DEVICE_TCA6424_CELL.init(EmbassyI2cDevice::new(i2c1_bus_mutex_ref));
    let mut tca6424_expander = Tca6424::new(&mut i2c_device_tca6424, tca6424::DEFAULT_ADDRESS).unwrap();
    info!("TCA6424 expander initialized.");

    // Configure P01 (Port 2 UFP) and P25 (Port 3 UFP) as inputs
    tca6424_expander.set_pin_direction(Pin::P01, PinDirection::Input).await.unwrap();
    tca6424_expander.set_pin_direction(Pin::P25, PinDirection::Input).await.unwrap();
    info!("TCA6424 P01 and P25 configured as inputs.");

    // Create I2cDevice instance for SW2303
    static I2C_DEVICE_SW2303_CELL: StaticCell<EmbassyI2cDevice<'static, CriticalSectionRawMutex, I2c<'static, mode::Async>>> = StaticCell::new();
    let mut i2c_device_sw2303 = I2C_DEVICE_SW2303_CELL.init(EmbassyI2cDevice::new(i2c1_bus_mutex_ref));
    let mut sw2303_controller = SW2303::new(&mut i2c_device_sw2303, sw2303::registers::constants::DEFAULT_ADDRESS);

    // Initialize SW2303 PD controller
    match sw2303_controller.init().await {
        Ok(_) => {
            info!("SW2303 PD controller initialized successfully.");

            // Configure SW2303 for 65W power (20V, 3.25A) with 100mA detection threshold
            match configure_sw2303_power(&mut sw2303_controller).await {
                Ok(_) => info!("SW2303 configured for 65W power with 100mA detection threshold."),
                Err(e) => {
                    error!("Failed to configure SW2303 power settings: {:?}", e);
                    // Continue with basic functionality
                }
            }
        },
        Err(e) => {
            error!("Failed to initialize SW2303: {:?}", e);
            // Continue without SW2303 functionality
        }
    }

    // Initialize buzzer PWM on TIM3_CH1 (PC6)
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
    // Start with buzzer off
    buzzer_pwm.ch1().set_duty_cycle_percent(0);
    info!("Buzzer PWM initialized on PC6 (TIM3_CH1).");

    // Test buzzer on startup
    info!("Testing buzzer...");
    beep_buzzer(&mut buzzer_pwm, 300).await; // 300ms test beep
    info!("Buzzer test complete.");

    struct EmbassyDisplayTimer;
    impl Gc9d01Timer for EmbassyDisplayTimer {
        async fn after_millis(milliseconds: u64) {
            embassy_time::Timer::after_millis(milliseconds).await;
        }
    }

    let spi_peripheral_instance = p.SPI1;
    impl embedded_hal::digital::ErrorType for EmbassyDisplayTimer {
        type Error = core::convert::Infallible;
    }
    let sck_pin = p.PB3;
    let mosi_pin = p.PA7;

    // According to compiler error E0107 (note), Output<'d> has 0 type generic arguments.
    // This contradicts embassy-stm32 source, but we follow the compiler error.
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

    // According to compiler error E0107 (note), Spi<'d, M: PeriMode> has 1 type generic argument M.
    // For async SPI1, M should be (peripherals::SPI1, mode::Async).

    static SPI_BUS_CELL: StaticCell<
        Mutex<CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>>,
    > = StaticCell::new();
    let spi_bus_mutex_ref = SPI_BUS_CELL.init(Mutex::new(spi_bus));

    // EmbassySpiDevice<'a, Mtx: RawMutex, BUS: SpiBus, CS: OutputPin>
    // CS type is now CsPinConcreteType = Output<'static>
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
    info!("Display initialization complete."); // Added log

    // Instantiate Dashboard
    let mut dashboard = Dashboard::new();

    display.fill_color(Rgb565::CSS_BLACK).await.unwrap();

    info!("Drawing test pattern.");
    let colors = [
        Rgb565::CSS_WHITE,
        Rgb565::CSS_YELLOW,
        Rgb565::CSS_CYAN,
        Rgb565::CSS_GREEN,
        Rgb565::CSS_MAGENTA,
        Rgb565::CSS_RED,
        Rgb565::CSS_BLUE,
        Rgb565::CSS_BLACK,
    ];

    // Each stripe is 5 pixels wide and 160 pixels high
    const STRIPE_WIDTH: u16 = 20;
    const STRIPE_HEIGHT: u16 = 40;

    // Create a buffer for one stripe's pixel data
    let mut stripe_pixels = [Rgb565::CSS_BLACK; (STRIPE_WIDTH * STRIPE_HEIGHT) as usize];

    for (i, color) in colors.iter().enumerate() {
        let x = i as u16 * STRIPE_WIDTH;

        // Fill the stripe buffer with the current color
        for pixel in stripe_pixels.iter_mut() {
            *pixel = *color;
        }

        // Write the pixel data for the current stripe
        display
            .write_area(x, 0, STRIPE_WIDTH, STRIPE_HEIGHT, &stripe_pixels)
            .await
            .unwrap();
    }

    // Initial delay before starting the loop
    embassy_time::Timer::after_secs(1).await;

    // Initialize previous UFP states for change detection
    let mut prev_port1_connected = false; // SW2303 Port 1
    let mut prev_port2_connected = false;
    let mut prev_port3_connected = false;

    loop {
        // Read data from INA226 sensors
        // Use correct async function names and handle Option<f64> return types
        let voltage1 = ina226_1.bus_voltage_millivolts().await.unwrap_or(0.0);
        let current1 = ina226_1.current_amps().await.unwrap_or(None).unwrap_or(0.0);
        let power1 = ina226_1.power_watts().await.unwrap_or(None).unwrap_or(0.0);

        let voltage2 = ina226_2.bus_voltage_millivolts().await.unwrap_or(0.0);
        let current2 = ina226_2.current_amps().await.unwrap_or(None).unwrap_or(0.0);
        let power2 = ina226_2.power_watts().await.unwrap_or(None).unwrap_or(0.0);

        let voltage3 = ina226_3.bus_voltage_millivolts().await.unwrap_or(0.0);
        let current3 = ina226_3.current_amps().await.unwrap_or(None).unwrap_or(0.0);
        let power3 = ina226_3.power_watts().await.unwrap_or(None).unwrap_or(0.0);

        // Read SW2303 sink device connection status for Port 1 (more reliable than UFP status)
        let sw2303_port1_connected = match sw2303_controller.is_sink_device_connected().await {
            Ok(connected) => connected,
            Err(e) => {
                error!("Failed to read SW2303 sink device status: {:?}", e);
                false
            }
        };

        // Read P2_UFP (P01) and P3_UFP (P25) states
        let p2_ufp_state = tca6424_expander.get_pin_input_state(Pin::P01).await.unwrap();
        let p3_ufp_state = tca6424_expander.get_pin_input_state(Pin::P25).await.unwrap();

        // Px_UFP is Low Active, so Low means connected
        let port2_connected = p2_ufp_state == PinState::Low;
        let port3_connected = p3_ufp_state == PinState::Low;

        // Check for UFP status changes and trigger buzzer
        if sw2303_port1_connected != prev_port1_connected {
            info!("SW2303 PD controller Port 1 UFP status changed: {} -> {}", prev_port1_connected, sw2303_port1_connected);
            beep_buzzer(&mut buzzer_pwm, 200).await; // 200ms beep
            prev_port1_connected = sw2303_port1_connected;
        }

        if port2_connected != prev_port2_connected {
            info!("TCA6424 Port 2 UFP status changed: {} -> {}", prev_port2_connected, port2_connected);
            beep_buzzer(&mut buzzer_pwm, 200).await; // 200ms beep
            prev_port2_connected = port2_connected;
        }

        if port3_connected != prev_port3_connected {
            info!("TCA6424 Port 3 UFP status changed: {} -> {}", prev_port3_connected, port3_connected);
            beep_buzzer(&mut buzzer_pwm, 200).await; // 200ms beep
            prev_port3_connected = port3_connected;
        }

        // Prepare data for Dashboard, converting f64 to f32
        let sensor_data = [
            ((voltage1 / 1000.0) as f32, current1 as f32, power1 as f32),
            ((voltage2 / 1000.0) as f32, current2 as f32, power2 as f32),
            ((voltage3 / 1000.0) as f32, current3 as f32, power3 as f32),
        ];

        // Prepare connection status for Dashboard
        let connection_status = [true, port2_connected, port3_connected]; // Assuming Port 1 is always connected or not relevant for this check

        // Update Dashboard data
        dashboard.update_data(sensor_data, connection_status);

        // Draw Dashboard directly to the display
        dashboard.draw(&mut display).await.unwrap();

        // Wait for 1 second before the next update
        embassy_time::Timer::after_millis(100).await;
    }
}
