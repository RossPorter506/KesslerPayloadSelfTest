#![no_main]
#![no_std]

use embedded_hal::digital::v2::*;
use msp430_rt::entry;
use msp430fr2x5x_hal::{gpio::Batch, pmm::Pmm, watchdog::Wdt};
use panic_msp430 as _;
use msp430;


mod pcb_mapping_v5;
mod spi;
mod dac;
use pcb_mapping_v5::{LEDPins};

#[entry]
fn main() -> ! {
    let periph = msp430fr2355::Peripherals::take().unwrap();
    let _wdt = Wdt::constrain(periph.WDT_A);

    let pmm = Pmm::new(periph.PMM);

    let port2 = Batch::new(periph.P2)
        .config_pin1(|p| p.to_output())
        .config_pin2(|p| p.to_output())
        .config_pin3(|p| p.to_output())
        .split(&pmm);

    let mut led_pins = LEDPins{red_led: port2.pin1, 
                                        yellow_led: port2.pin2, 
                                        green_led: port2.pin3};

    let mut counter: u8 = 0;

    loop {
        snake_leds(&mut counter, &mut led_pins);
        delay_cycles(15000);
    }
}

fn delay_cycles(num_cycles: usize){ //approximate delay fn
    for _ in 0..num_cycles/2 {
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
