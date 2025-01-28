#![no_main]
#![no_std]
#![allow(dead_code, unused_variables, unused_imports)] // TODO: Remove when ready

#![allow(clippy::upper_case_acronyms, clippy::needless_return)]

#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(const_trait_impl)]

use embedded_hal::digital::v2::*;
use msp430_rt::entry;
use msp430fr2355::{P1, P2, P3, P4, P5, P6, PMM};
#[allow(unused_imports)]
use msp430fr2x5x_hal::{gpio::Batch, pmm::Pmm, watchdog::Wdt, serial::{SerialConfig, StopBits, BitOrder, BitCount, Parity, Loopback, SerialUsci}, clock::{ClockConfig, DcoclkFreqSel, MclkDiv}, fram::Fram};
#[allow(unused_imports)]
use ufmt::{uwrite, uwriteln};

#[cfg(debug_assertions)]
use panic_msp430 as _;

#[cfg(not(debug_assertions))]
use panic_never as _;

pub mod pcb_common; // pcb_mapping re-exports these values, so no need to interact with this file.
// This line lets every other file do 'use pcb_mapping', we only have to change the version once here.
mod pcb_mapping { include!("pcb_v7_mapping.rs"); }

use pcb_mapping::{PayloadControlPins, PayloadSPIBitBangPins, DebugSerialPins, LEDPins, PinpullerActivationPins, TetherLMSPins, DeploySensePins, PayloadPeripherals, PayloadSPIChipSelectPins, power_supply_limits::HEATER_MIN_VOLTAGE_MILLIVOLTS};
mod spi; use spi::{PayloadSPIController, PayloadSPI, SckPhase::SampleFirstEdge, SckPolarity::IdleLow};
mod dac; use dac::DAC;
mod adc; use adc::{ApertureADC, MiscADC, TemperatureADC, TetherADC};
mod digipot; use digipot::Digipot;
mod payload; use payload::PayloadBuilder;
mod serial; use serial::SerialWriter;

#[allow(unused_imports)]
mod testing; use testing::{AutomatedFunctionalTests, AutomatedPerformanceTests, ManualFunctionalTests, ManualPerformanceTests};

#[allow(unused_mut)]
#[entry]
fn main() -> ! {
    if let Some(periph) = msp430fr2355::Peripherals::take() {
        let _wdt = Wdt::constrain(periph.WDT_A);
        
        let (payload_spi_pins, 
            mut pinpuller_pins, 
            mut led_pins, 
            mut payload_control_pins, 
            mut lms_control_pins, 
            mut deploy_sense_pins, 
            mut payload_peripheral_cs_pins, 
            debug_serial_pins) = collect_pins(periph.PMM, periph.P1, periph.P2, periph.P3, periph.P4, periph.P5, periph.P6);
        
        lms_control_pins.lms_led_enable.set_low().ok();
        led_pins.green_led.toggle().ok();
        payload_peripheral_cs_pins.dac.set_high().ok();
        payload_control_pins.payload_enable.set_high().ok(); // Enable payload so DAC can hear it's reference selection that happens during collection
        delay_cycles(100_000);
        
        // As the bus's idle state is part of it's type, peripherals will not accept an incorrectly configured bus
        // The SPI controller handles all of this for us. All we need to do is call .borrow() to get a mutable reference to it
        let mut payload_spi_controller = PayloadSPIController::new(payload_spi_pins);

        // Collate peripherals into a single struct
        let payload_peripherals = collect_payload_peripherals(payload_peripheral_cs_pins, &mut payload_spi_controller);
        // Create an object to manage payload state
        let mut payload = PayloadBuilder::build(payload_peripherals, payload_control_pins).into_enabled_payload();
        
        let mut fram = Fram::new(periph.FRCTL);

        let (smclk, aclk) = ClockConfig::new(periph.CS)
            .mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
            .smclk_on(msp430fr2x5x_hal::clock::SmclkDiv::_1)
            .freeze(&mut fram);
        for _ in 0..2 {msp430::asm::nop();} // seems to be some weird bug with clock selection. MSP hangs in release mode when this is removed.

        led_pins.yellow_led.toggle().ok();

        let (serial_tx_pin, mut serial_rx_pin) = SerialConfig::new(  
            periph.E_USCI_A1,
            BitOrder::LsbFirst,
            BitCount::EightBits,
            StopBits::OneStopBit,
            Parity::NoParity,
            Loopback::NoLoop,
            9600)
            .use_aclk(&aclk)
            .split(debug_serial_pins.tx, debug_serial_pins.rx);

        led_pins.red_led.toggle().ok();
        // Wrapper struct so we can use ufmt traits like uwrite! and uwriteln!
        let mut serial_writer = SerialWriter::new(serial_tx_pin);

        payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS, &mut payload_spi_controller);
        let mut payload = payload.into_enabled_heater();
        
        AutomatedFunctionalTests::full_system_test(&mut payload, &mut pinpuller_pins, &mut lms_control_pins, &mut payload_spi_controller, &mut serial_writer);
        AutomatedPerformanceTests::full_system_test(&mut payload, &mut pinpuller_pins, &mut payload_spi_controller, &mut serial_writer);
        //ManualFunctionalTests::full_system_test(&mut deploy_sense_pins, &mut serial_writer, &mut serial_rx_pin);
        //ManualPerformanceTests::test_heater_voltage(&mut payload, &mut payload_spi_controller, &mut serial_writer, &mut serial_rx_pin);

        let mut payload = payload.into_disabled_heater().into_disabled_payload();
        idle_loop(&mut led_pins);
    }
    else {#[allow(clippy::empty_loop)] loop{}}
}

fn idle_loop(led_pins: &mut LEDPins) -> ! {
    let mut counter: u8 = 0;
    loop {
        snake_leds(&mut counter, led_pins);
        //uwrite!(serial_writer, "Hello, World!\r\n").ok();
        delay_cycles(45000);
    }
}

fn delay_cycles(num_cycles: u32){ //approximate delay fn
    let delay = (6*num_cycles)/128;
    for _ in 0..delay {
        msp430::asm::nop()
    }
}

fn snake_leds(n: &mut u8, led_pins: &mut LEDPins){
    *n = (*n + 1) % 4;
    match n {
        1 => led_pins.green_led.toggle().ok(),
        2 => led_pins.yellow_led.toggle().ok(),
        3 => led_pins.red_led.toggle().ok(),
        _ => Some(()),
    };
}

fn collect_payload_peripherals(cs_pins: PayloadSPIChipSelectPins, payload_spi_bus: &mut PayloadSPIController) -> PayloadPeripherals{
    // Note that the peripherals gain ownership of their associated pins
    let digipot = Digipot::new(cs_pins.digipot);
    let dac = DAC::new(cs_pins.dac, payload_spi_bus.borrow());
    let tether_adc = TetherADC::new(cs_pins.tether_adc);
    let temperature_adc = TemperatureADC::new(cs_pins.temperature_adc);
    let misc_adc = MiscADC::new(cs_pins.misc_adc);
    let aperture_adc = ApertureADC::new(cs_pins.aperture_adc);
    PayloadPeripherals { digipot, dac, tether_adc, temperature_adc, misc_adc, aperture_adc }
}

// Takes raw port peripherals and returns actually useful pin collections 
fn collect_pins(pmm: PMM, p1: P1, p2: P2, p3: P3, p4: P4, p5: P5, p6: P6) -> (
    PayloadSPIBitBangPins,
    PinpullerActivationPins,
    LEDPins,
    PayloadControlPins,
    TetherLMSPins,
    DeploySensePins,
    PayloadSPIChipSelectPins,
    DebugSerialPins){

let pmm = Pmm::new(pmm);
let port1 = Batch::new(p1).split(&pmm);
let port2 = Batch::new(p2).split(&pmm);
let port3 = Batch::new(p3).split(&pmm);
let port4 = Batch::new(p4).split(&pmm);
let port5 = Batch::new(p5).split(&pmm);
let port6 = Batch::new(p6).split(&pmm);

let payload_spi_pins = PayloadSPIBitBangPins {
    miso: port4.pin7.pullup(),
    mosi: port4.pin6.to_output(),
    sck:  port4.pin5.to_output(),};

let pinpuller_pins = PinpullerActivationPins{ 
    burn_wire_1:        port3.pin2.to_output(),
    burn_wire_1_backup: port3.pin3.to_output(),
    burn_wire_2:        port5.pin0.to_output(),
    burn_wire_2_backup: port5.pin1.to_output(),};

let led_pins = LEDPins{
    red_led: port2.pin1.to_output(), 
    yellow_led: port2.pin2.to_output(), 
    green_led: port2.pin3.to_output(),};

let payload_control_pins = PayloadControlPins{   
    payload_enable: port6.pin6.to_output(),
    heater_enable: port4.pin4.to_output(), 
    cathode_switch: port3.pin0.to_output(), 
    tether_switch: port6.pin1.to_output(),};

let lms_control_pins = TetherLMSPins{   
    lms_receiver_enable: port3.pin4.to_output(), 
    lms_led_enable:      port3.pin5.to_output(),};

let deploy_sense_pins = DeploySensePins{
    endmass_sense_1: port5.pin2.pulldown(),
    endmass_sense_2: port3.pin1.pulldown(),
    pinpuller_sense: port5.pin3.pullup()};

    // in lieu of stateful output pins, constructor sets all pins high, 
let payload_peripheral_cs_pins = PayloadSPIChipSelectPins::new(
    port6.pin4.to_output(), 
    port6.pin3.to_output(), 
    port6.pin2.to_output(), 
    port6.pin0.to_output(), 
    port5.pin4.to_output(), 
    port1.pin3.to_output());

let debug_serial_pins = DebugSerialPins{
    rx: port4.pin2.to_output().to_alternate1(),
    tx: port4.pin3.to_output().to_alternate1(),};

(payload_spi_pins, pinpuller_pins, led_pins, payload_control_pins, lms_control_pins, deploy_sense_pins, payload_peripheral_cs_pins, debug_serial_pins)
}

// The compiler will emit calls to the abort() compiler intrinsic if debug assertions are
// enabled (default for dev profile). MSP430 does not actually have meaningful abort() support
// so for now, we create our own in each application where debug assertions are present.
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
