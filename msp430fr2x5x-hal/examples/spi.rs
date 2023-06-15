#![no_main]
#![no_std]

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::*;
use msp430_rt::entry;
use msp430fr2x5x_hal::{
    clock::{ClockConfig, DcoclkFreqSel, MclkDiv, SmclkDiv},
    fram::Fram,
    gpio::Batch,
    pmm::Pmm,
    spi::*,
    serial::*,
    watchdog::Wdt,
};

use nb::block;

#[cfg(debug_assertions)]
use panic_msp430 as _;

#[cfg(not(debug_assertions))]
use panic_never as _;

// Configures the eUSCI_A0 peripheral for SPI, prints "HELLO" to (theoretical) device 1, then echos to (theoretical) device 2.
// Spi settings are listed in the code
#[entry]
fn main() -> ! {
    if let Some(periph) = msp430fr2355::Peripherals::take() {
        let mut fram = Fram::new(periph.FRCTL);
        let _wdt = Wdt::constrain(periph.WDT_A);

        let (smclk, aclk) = ClockConfig::new(periph.CS)
            .mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
            .smclk_on(SmclkDiv::_1)
            //.aclk_refoclk()
            .freeze(&mut fram);

        let pmm = Pmm::new(periph.PMM);
        let p1 = Batch::new(periph.P1).split(&pmm);
        let p4 = Batch::new(periph.P4).split(&pmm);
        let mut led = p1.pin0.to_output();
        led.set_low().ok();

        let mut chip_select_1 = p1.pin1.to_output();
        chip_select_1.set_high().ok();
        let mut chip_select_2 = p1.pin2.to_output();
        chip_select_2.set_high().ok();

        // Configure SPI bus parameters
        let mut spi_bus = SpiConfig::new(
            periph.E_USCI_B1,
            Polarity::IdleHigh, Phase::CaptureOnFirstEdge,
            BitOrder::MsbFirst, BitCount::EightBits,
            Loopback::NoLoop, 250_000)
            .use_smclk(&smclk)
            .apply_config(
                p4.pin5.to_output().to_alternate1(),  
                p4.pin7.to_output().to_alternate1(),
                p4.pin6.to_output().to_alternate1());

        let (mut tx, _rx) = SerialConfig::new(
            periph.E_USCI_A1,
            BitOrder::LsbFirst,
            BitCount::EightBits,
            StopBits::OneStopBit,
            // Launchpad UART-to-USB converter doesn't handle parity, so we don't use it
            Parity::NoParity,
            Loopback::NoLoop,
            9600)
            .use_aclk(&aclk)
            .split(p4.pin3.to_alternate1(), p4.pin2.to_alternate1());

        led.set_high().ok(); //Configuration complete!

        let mut buf: [u8; 2] = [0,0];
        spi_bus.transfer(&mut buf).ok();

        
        let digit_arr = u8_to_ascii(((buf[0] as u16) << 8) + buf[1] as u16);
        
        block!(tx.write(digit_arr[0])).ok();
        block!(tx.write(digit_arr[1])).ok();
        block!(tx.write(digit_arr[2])).ok();
        block!(tx.write(digit_arr[3])).ok();
        block!(tx.write(digit_arr[4])).ok();
        block!(tx.write(digit_arr[5])).ok();

        // Begin transmission to device 1.
        /*chip_select_1.set_low().ok();
        let mut buf: [u8; 5] = *b"HELLO";
        spi_bus.transfer(&mut buf).ok();

        // Ensure that transmission has finished before we de-assert CS and reconfigure the bus.
        block!(spi_bus.flush()).ok();

        chip_select_1.set_high().ok();
        spi_bus.reconfigure(
            Polarity::IdleLow, Phase::CaptureOnSecondEdge,
            BitOrder::MsbFirst, BitCount::EightBits
        );

        chip_select_2.set_low().ok();
        loop {
            
            let ch = match block!(spi_bus.read()) {
                Ok(c) => c,
                Err(SpiError::Overrun(_)) => '!' as u8,
                Err(SpiError::Framing) => '?' as u8,
            };
            block!(spi_bus.send(ch)).ok();
        }*/
        // chip_select_2.set_high().ok();

        // Once you are done, you can consume the bus to release the hardware pins.
        // let (port1pin5, _port1pin6, _port1pin7) = spi_bus.return_pins();
        // port1pin5.to_gpio().set_high().ok();
        loop {}
    } else {
        loop {}
    }
}
fn u16_to_ascii(mut num: u16) -> [u8;6] {
    let mut ascii_arr: [u8;6] = [b'0';6];
    for i in 0..=5 {
        let digit: u8 = (num % 10) as u8;
        ascii_arr[5-i] = digit + b'0';
        num = num / 10;
    }
    ascii_arr
}

// The compiler will emit calls to the abort() compiler intrinsic if debug assertions are
// enabled (default for dev profile). MSP430 does not actually have meaningful abort() support
// so for now, we create our own in each application where debug assertions are present.
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
