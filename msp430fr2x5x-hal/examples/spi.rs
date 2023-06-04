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
    watchdog::Wdt,
};
use nb::block;

#[cfg(debug_assertions)]
use panic_msp430 as _;

#[cfg(not(debug_assertions))]
use panic_never as _;

// Prints "HELLO" when started then echos on euSCI_A0.
// Spi settings are listed in the code
#[entry]
fn main() -> ! {
    if let Some(periph) = msp430fr2355::Peripherals::take() {
        let mut fram = Fram::new(periph.FRCTL);
        let _wdt = Wdt::constrain(periph.WDT_A);

        let (smclk, _aclk) = ClockConfig::new(periph.CS)
            .mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
            .smclk_on(SmclkDiv::_1)
            .aclk_refoclk()
            .freeze(&mut fram);

        let pmm = Pmm::new(periph.PMM);
        let p1 = Batch::new(periph.P1).split(&pmm);
        let mut led = p1.pin0.to_output();
        led.set_low().ok();

        let mut spi_bus = SpiConfig::new(
            periph.E_USCI_A0,
            Polarity::IdleHigh, Phase::CaptureOnFirstEdge,
            BitOrder::MsbFirst, BitCount::EightBits,
            Loopback::NoLoop, 250_000)
            .use_smclk(&smclk)
            .apply_config(p1.pin5.to_alternate1(), p1.pin6.to_alternate1(), p1.pin7.to_alternate1());

        led.set_high().ok();
        let mut buf: [u8; 6] = *b"HELLO\n";
        spi_bus.transfer(&mut buf).ok();

        spi_bus.reconfigure(
            Polarity::IdleLow, Phase::CaptureOnSecondEdge,
            BitOrder::MsbFirst, BitCount::EightBits
        );

        loop {
            let ch = match block!(spi_bus.read()) {
                Ok(c) => c,
                Err(SpiError::Overrun(_)) => '!' as u8,
                Err(SpiError::Framing) => '?' as u8,
            };
            block!(spi_bus.send(ch)).ok();
        }
    } else {
        loop {}
    }
}

// The compiler will emit calls to the abort() compiler intrinsic if debug assertions are
// enabled (default for dev profile). MSP430 does not actually have meaningful abort() support
// so for now, we create our own in each application where debug assertions are present.
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
