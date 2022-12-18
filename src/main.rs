#![no_main]
#![no_std]
#![feature(int_log)]
#![allow(dead_code, unused_variables)] // TODO: Remove when ready

use digipot::Digipot;
use embedded_hal::{digital::v2::*};
use msp430_rt::entry;
use msp430fr2x5x_hal::{gpio::Batch, pmm::Pmm, watchdog::Wdt, serial::{SerialConfig, StopBits, BitOrder, BitCount, Parity, Loopback}, clock::{ClockConfig, SmclkDiv, DcoclkFreqSel, MclkDiv}, fram::Fram};
use panic_msp430 as _;
use msp430;
#[allow(unused_imports)]
use testing::{AutomatedFunctionalTests, AutomatedPerformanceTests};
use ufmt::uwrite;

mod pcb_mapping_v5; use pcb_mapping_v5::{LEDPins, PinpullerActivationPins, TetherLMSPins, DeploySensePins};
mod spi; use spi::{PayloadSPIBitBangConfig};
mod dac; use dac::{DAC};
mod adc; use adc::{TetherADC,TemperatureADC,MiscADC};
mod digipot;
mod sensors;
mod serial; use serial::SerialWriter;

use crate::{pcb_mapping_v5::PayloadControlPins, sensors::PayloadBuilder};
mod testing;


#[allow(unused_mut)]
#[entry]
fn main() -> ! {
    let periph = msp430fr2355::Peripherals::take().unwrap();
    let _wdt = Wdt::constrain(periph.WDT_A);

    let pmm = Pmm::new(periph.PMM);
    

    let port2 = Batch::new(periph.P2).split(&pmm);
    let port3 = Batch::new(periph.P3).split(&pmm);
    let port4 = Batch::new(periph.P4).split(&pmm);
    let port5 = Batch::new(periph.P5).split(&pmm);
    let port6 = Batch::new(periph.P6).split(&pmm);
    
    let mut pinpuller_pins = PinpullerActivationPins{ 
        burn_wire_1:        port3.pin2.to_output(),
        burn_wire_1_backup: port3.pin3.to_output(),
        burn_wire_2:        port5.pin0.to_output(),
        burn_wire_2_backup: port5.pin1.to_output()};

    let mut led_pins = LEDPins{
        red_led: port2.pin1.to_output(), 
        yellow_led: port2.pin2.to_output(), 
        green_led: port2.pin3.to_output()};
    
    let mut payload_spi_bus = PayloadSPIBitBangConfig::new( 
        port4.pin7.pulldown(),
        port4.pin6.to_output(),
        port4.pin5.to_output(),)
        .sck_idle_low()
        .sample_on_first_edge()
        .create();
    
    let mut payload_control = PayloadControlPins{   
        payload_enable: port6.pin6.to_output(),
        heater_enable: port4.pin4.to_output(), 
        cathode_switch: port3.pin0.to_output(), 
        tether_switch: port6.pin1.to_output() };

    let mut lms_control_pins = TetherLMSPins{   
        lms_receiver_enable: port3.pin4.to_output(), 
        lms_led_enable:      port3.pin5.to_output()};

    let mut deploy_sense_pins = DeploySensePins{
        endmass_sense_1: port5.pin2.pulldown(),
        endmass_sense_2: port3.pin1.pulldown(),
        pinpuller_sense: port5.pin3.pullup()};
    
    let mut digipot = Digipot::new(port6.pin4.to_output());
    let mut dac = DAC::new(port6.pin3.to_output(), &mut payload_spi_bus);
    let mut tether_adc = TetherADC::new(port6.pin2.to_output());
    let mut temperature_adc = TemperatureADC::new(port6.pin0.to_output());
    let mut misc_adc = MiscADC::new(port5.pin4.to_output());
    
    // As the bus's idle state is part of it's type, peripherals will not accept an incorrectly configured bus
    //let mut payload_spi_bus = payload_spi_bus.into_sck_idle_high();
    //tether_adc.read_count_from(&REPELLER_VOLTAGE_SENSOR, &mut payload_spi_bus); // Ok, the ADC wants an idle high SPI bus.
    //dac.send_command(dac::DACCommand::NoOp, DACChannel::ChannelA, 0x000, &mut payload_spi_bus); // Compile error! DAC expects a bus that idles low.

    let mut fram = Fram::new(periph.FRCTL);
    let (smclock, _aclock) = ClockConfig::new(periph.CS).mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
                                                                     .smclk_on(SmclkDiv::_1)
                                                                     .freeze(&mut fram);
    let (serial_tx, serial_rx) = SerialConfig::new(  
            periph.E_USCI_A1,
            BitOrder::LsbFirst,
            BitCount::EightBits,
            StopBits::OneStopBit,
            Parity::NoParity,
            Loopback::NoLoop,
            115200)
            .use_smclk(&smclock).split(port4.pin3.to_alternate1(), port4.pin2.to_alternate1());

    let mut serial_writer = SerialWriter::new(serial_tx);

    let mut payload = PayloadBuilder::new_enabled_payload(tether_adc, temperature_adc, misc_adc, dac, digipot, payload_control);

    let mut payload_spi_bus = payload_spi_bus.into_idle_high();

    //AutomatedFunctionalTests::full_system_test(&mut payload, &mut pinpuller_pins, &mut lms_control_pins, &mut payload_spi_bus, &mut serial_writer);
    //AutomatedPerformanceTests::full_system_test(&mut payload, &mut pinpuller_pins, &mut payload_spi_bus, &mut serial_writer);
    AutomatedPerformanceTests::test_heater(&mut payload, &mut payload_spi_bus);
    //AutomatedPerformanceTests::test_cathode_offset(&mut payload, &mut payload_spi_bus);
    let mut counter: u8 = 0;

    loop {
        snake_leds(&mut counter, &mut led_pins);
        uwrite!(serial_writer, "Hello, World!\r\n").ok();
        delay_cycles(45000);
    }
}

fn delay_cycles(num_cycles: u32){ //approximate delay fn
    let delay = (19*num_cycles + 13*num_cycles)/32;
    for _ in 0..delay {
        msp430::asm::nop()
    }
}

fn snake_leds(n: &mut u8, led_pins: &mut LEDPins){
    match n {
        1 => led_pins.green_led.toggle().ok(),
        2 => led_pins.yellow_led.toggle().ok(),
        3 => led_pins.red_led.toggle().ok(),
        _ => Some(()),
    };
    *n = (*n + 1) % 4;
}

// The compiler will emit calls to the abort() compiler intrinsic if debug assertions are
// enabled (default for dev profile). MSP430 does not actually have meaningful abort() support
// so for now, we create our own in each application where debug assertions are present.
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
