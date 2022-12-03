#![no_main]
#![no_std]

#![allow(dead_code, unused_variables)] // TODO: Remove when ready

use adc::{TetherADC,TemperatureADC,MiscADC, ADCChannel, TemperatureSensor, MiscSensor};
use digipot::Digipot;
use embedded_hal::digital::v2::*;
use msp430_rt::entry;
use msp430fr2x5x_hal::{gpio::Batch, pmm::Pmm, watchdog::Wdt};
use panic_msp430 as _;
use msp430;

mod pcb_mapping_v5;
mod spi;
mod dac;
mod adc;
mod digipot;
mod sensors;
use pcb_mapping_v5::{LEDPins, PayloadSPIPins};
use spi::{PayloadSPIBitBang};
use dac::DAC;

#[allow(unused_mut)]
#[entry]
fn main() -> ! {
    let periph = msp430fr2355::Peripherals::take().unwrap();
    let _wdt = Wdt::constrain(periph.WDT_A);

    let pmm = Pmm::new(periph.PMM);

    let port2 = Batch::new(periph.P2).split(&pmm);
    let port4 = Batch::new(periph.P4).split(&pmm);
    let port5 = Batch::new(periph.P5).split(&pmm);
    let port6 = Batch::new(periph.P6).split(&pmm);
    

    let mut led_pins = LEDPins{red_led: port2.pin1.to_output(), 
                                        yellow_led: port2.pin2.to_output(), 
                                        green_led: port2.pin3.to_output()};
    
    let mut payload_spi_bus = PayloadSPIBitBang::new(
        PayloadSPIPins{ miso: port4.pin7.to_output().to_alternate1(),
                              mosi: port4.pin6.to_output().to_alternate1(),
                              sck:  port4.pin5.to_output().to_alternate1()});

    let mut digipot = Digipot::new(port6.pin4.to_output());
    let mut dac = DAC::new(port6.pin3.to_output(), &mut payload_spi_bus);
    let mut tether_adc = TetherADC::new(port6.pin2.to_output());
    let mut temperature_adc = TemperatureADC::new(port6.pin0.to_output());
    let mut misc_adc = MiscADC::new(port5.pin4.to_output());

    let sensor_a = TemperatureSensor{channel: ADCChannel::IN0};
    let sensor_b = MiscSensor{channel: ADCChannel::IN0};

    //misc_adc.read_voltage_from(&sensor_a, &mut payload_spi_bus); // should compile error
    misc_adc.read_voltage_from(&sensor_b, &mut payload_spi_bus);

    let mut counter: u8 = 0;

    loop {
        snake_leds(&mut counter, &mut led_pins);
        delay_cycles(15000);
    }
}

fn delay_cycles(num_cycles: usize){ //approximate delay fn
    for _ in 0..num_cycles/3 {
        msp430::asm::nop()
    }
}

fn snake_leds(n: &mut u8, led_pins: &mut LEDPins){
    match n {
        1 => led_pins.red_led.toggle().ok(),
        2 => led_pins.yellow_led.toggle().ok(),
        3 => led_pins.green_led.toggle().ok(),
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
