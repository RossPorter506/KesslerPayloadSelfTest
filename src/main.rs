#![no_main]
#![no_std]

#![allow(dead_code, unused_variables)] // TODO: Remove when ready


use digipot::Digipot;
use embedded_hal::{digital::v2::*};
use msp430_rt::entry;
use msp430fr2x5x_hal::{gpio::Batch, pmm::Pmm, watchdog::Wdt, serial::{SerialConfig, StopBits, BitOrder, BitCount, Parity, Loopback}, clock::{ClockConfig, SmclkDiv, DcoclkFreqSel, MclkDiv}, fram::Fram};
use panic_msp430 as _;
use msp430;
use ufmt::uwrite;

mod pcb_mapping_v5; use pcb_mapping_v5::{LEDPins, PayloadSPIPins, PinpullerPins};
mod spi; use spi::{PayloadSPIBitBang};
mod dac; use dac::{DAC};
mod adc; use adc::{TetherADC,TemperatureADC,MiscADC};
mod digipot;
mod sensors;
mod serial; use serial::SerialWriter;
mod testing;

use crate::{spi::SckIdleLow, };

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
    
    let mut pinpuller_pins = PinpullerPins{ 
                                                burn_wire_1:        port3.pin2.to_output(),
                                                burn_wire_1_backup: port3.pin3.to_output(),
                                                burn_wire_2:        port5.pin0.to_output(),
                                                burn_wire_2_backup: port5.pin1.to_output(),
                                                pinpuller_sense:    port5.pin3.pullup(),
                                            };

    let mut led_pins = LEDPins{red_led: port2.pin1.to_output(), 
                                        yellow_led: port2.pin2.to_output(), 
                                        green_led: port2.pin3.to_output()};
    
    let mut payload_spi_bus:PayloadSPIBitBang<SckIdleLow> = PayloadSPIBitBang::<SckIdleLow>::new_idle_low_bus(
        PayloadSPIPins{miso: port4.pin7.to_output().to_alternate1(),
                            mosi: port4.pin6.to_output().to_alternate1(),
                            sck:  port4.pin5.to_output().to_alternate1()});

    let mut digipot = Digipot::new(port6.pin4.to_output());
    let mut dac = DAC::new(port6.pin3.to_output(), &mut payload_spi_bus);
    let mut tether_adc = TetherADC::new(port6.pin2.to_output());
    let mut temperature_adc = TemperatureADC::new(port6.pin0.to_output());
    let mut misc_adc = MiscADC::new(port5.pin4.to_output());
    
    // As the bus's idle state is part of it's type, peripherals will not accept an incorrectly configured bus
    //let mut payload_spi_bus = payload_spi_bus.into_sck_idle_high();
    //tether_adc.read_count_from(&REPELLER_VOLTAGE_SENSOR, &mut payload_spi_bus); // Ok, the ADC wants an idle high SPI bus.
    //dac.send_command(dac::DACCommand::NoOp, DACChannel::ChannelA, 0x000, &mut payload_spi_bus); // Compile error! DAC expects a bus that idles low.
    
    
    // Currently unused high-level interface
    //let mut payload = PayloadController::new(tether_adc, temperature_adc, misc_adc, dac, digipot);

    let mut fram = Fram::new(periph.FRCTL);
    let (smclock, _aclock) = ClockConfig::new(periph.CS).mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
                                                                     .smclk_on(SmclkDiv::_1)
                                                                     .freeze(&mut fram);
    let mut serial_tx = SerialConfig::new(  
                                periph.E_USCI_A1,
                                BitOrder::LsbFirst,
                                BitCount::EightBits,
                                StopBits::OneStopBit,
                                Parity::NoParity,
                                Loopback::NoLoop,
                                115200)
                                .use_smclk(&smclock)
                                .tx_only(port4.pin3.to_alternate1());

    let mut serial_writer = SerialWriter::new(serial_tx);

    let mut counter: u8 = 0;

    loop {
        snake_leds(&mut counter, &mut led_pins);
        uwrite!(serial_writer, "Hello, World!\r\n").unwrap();
        delay_cycles(45000);
    }
}

fn delay_cycles(num_cycles: u32){ //approximate delay fn
    let delay = num_cycles/19;
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
