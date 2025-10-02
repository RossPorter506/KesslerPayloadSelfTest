#![no_main]
#![no_std]
#![allow(dead_code, unused_variables, unused_imports)] // TODO: Remove when ready
#![allow(clippy::upper_case_acronyms, clippy::needless_return)]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(const_trait_impl)]

use core::{
    cell::{RefCell, UnsafeCell},
    ops::DerefMut,
};

use critical_section::{with, CriticalSection, Mutex};
use embedded_hal::{
    blocking::i2c,
    digital::v2::*,
    timer::{self, CountDown},
};
use msp430_rt::entry;
use msp430fr2355::{E_USCI_A1, P1, P2, P3, P4, P5, P6, PMM};
use msp430fr2x5x_hal::{
    clock::{ClockConfig, DcoclkFreqSel, MclkDiv},
    fram::Fram,
    gpio::Batch,
    pmm::Pmm,
    rtc::{Rtc, RtcDiv},
    serial::{BitCount, BitOrder, Loopback, Parity, SerialConfig, SerialUsci, StopBits},
    timer::{CapCmpTimer3, TBxIV, Timer, TimerConfig, TimerParts3},
    watchdog::Wdt,
};
use nb::block;
use tvac::tvac_test;
use ufmt::{uwrite, uwriteln};

#[cfg(debug_assertions)]
use panic_msp430 as _;

#[cfg(not(debug_assertions))]
use panic_never as _;

pub mod pcb_common; // pcb_mapping re-exports these values, so no need to interact with this file.
                    // This line lets every other file do 'use pcb_mapping', we only have to change the version once here.
mod pcb_mapping {
    include!("pcb_v7_mapping.rs");
}

use pcb_mapping::{
    power_supply_limits::HEATER_MIN_VOLTAGE_MILLIVOLTS, DebugSerialPins, DeploySensePins, LEDPins,
    PayloadControlPins, PayloadPeripherals, PayloadSPIBitBangPins, PayloadSPIChipSelectPins,
    PinpullerActivationPins, TetherLMSPins,
};
mod spi;
use spi::{PayloadSPI, PayloadSPIController, SckPhase::SampleFirstEdge, SckPolarity::IdleLow};
mod dac;
use dac::DAC;
mod adc;
use adc::{ApertureADC, MiscADC, TemperatureADC, TetherADC};
mod digipot;
use digipot::Digipot;
mod payload;
use payload::{
    HeaterState, HeaterState::*, PayloadBuilder, PayloadState, PayloadState::*, SwitchState,
};
mod serial;
use serial::SerialWriter;
mod tvac;

#[allow(unused_imports)]
mod testing;
use testing::{
    AutomatedFunctionalTests, AutomatedPerformanceTests, ManualFunctionalTests,
    ManualPerformanceTests,
};
use void::ResultVoidExt;

use crate::{
    payload::Payload,
    pcb_mapping::power_supply_limits::{
        CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS, HEATER_MAX_VOLTAGE_MILLIVOLTS,
        TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS,
    },
};

#[allow(unused_mut)]
#[entry]
fn main() -> ! {
    let mut board = configure_board();

    let mut board = board.into_enabled_payload();
    let mut board = board.into_enabled_heater();

    ManualPerformanceTests::test_cathode_offset_current(&mut board);

    idle_loop(&mut board.led_pins)
}

/// Take and configure MCU peripherals
fn configure_board() -> Payload<{ PayloadOff }, { HeaterOff }> {
    let Some(regs) = msp430fr2355::Peripherals::take() else {
        loop {}
    }; 
    let _wdt = Wdt::constrain(regs.WDT_A);

    let (
        payload_spi_pins,
        pinpuller_pins,
        mut led_pins,
        payload_control_pins,
        mut lms_control_pins,
        deploy_sense_pins,
        payload_peripheral_cs_pins,
        debug_serial_pins,
    ) = collect_pins(
        regs.PMM, regs.P1, regs.P2, regs.P3, regs.P4, regs.P5, regs.P6,
    );

    lms_control_pins.lms_led_enable.set_low().ok();
    led_pins.green_led.toggle().ok();
    delay_cycles(100_000);

    // As the bus's idle state is part of it's type, peripherals will not accept an incorrectly configured bus
    // The SPI controller handles all of this for us. All we need to do is call .borrow() to get a mutable reference to it
    let payload_spi_controller = PayloadSPIController::new(payload_spi_pins);

    // Collate peripherals into a single struct
    let payload_peripherals = collect_payload_peripherals(payload_peripheral_cs_pins);

    let mut fram = Fram::new(regs.FRCTL);

    // Clock selection
    let (smclk, aclk, delay) = ClockConfig::new(regs.CS)
        .mclk_dcoclk(DcoclkFreqSel::_1MHz, MclkDiv::_1)
        .smclk_on(msp430fr2x5x_hal::clock::SmclkDiv::_1)
        .freeze(&mut fram);
    msp430::asm::nop();

    led_pins.yellow_led.toggle().ok();

    // Timer configuration
    let parts = TimerParts3::new(regs.TB0, TimerConfig::aclk(&aclk));
    let timer = parts.timer;

    // Serial configuration
    let (serial_tx_pin, serial_reader) = SerialConfig::new(
        regs.E_USCI_A1,
        BitOrder::LsbFirst,
        BitCount::EightBits,
        StopBits::OneStopBit,
        Parity::NoParity,
        Loopback::NoLoop,
        115200,
    )
    .use_smclk(&smclk)
    .split(debug_serial_pins.tx, debug_serial_pins.rx);

    // Create an object to manage payload state
    led_pins.red_led.toggle().ok();
    let payload = PayloadBuilder::build(
        payload_peripherals,
        payload_control_pins,
        payload_spi_controller,
        pinpuller_pins,
        lms_control_pins,
        deploy_sense_pins,
        serial_reader,
        led_pins,
        timer,
    );

    // Wrapper struct so we can use ufmt traits like uwrite! and uwriteln!
    let serial_writer = SerialWriter::new(serial_tx_pin);

    // Move serial_writer into a static variable so we can print from anywhere without having to carry it around
    critical_section::with(|cs| {
        unsafe { &mut *crate::serial::SERIAL_WR.borrow(cs).get() }.replace(serial_writer);
    });

    println!("Hello world!");

    payload
}

fn idle_loop(led_pins: &mut LEDPins) -> ! {
    let mut counter: u8 = 0;
    loop {
        snake_leds(&mut counter, led_pins);
        //uwrite!(serial_writer, "Hello, World!\r\n").ok();
        delay_cycles(45000);
    }
}

fn delay_cycles(num_cycles: u32) {
    //approximate delay fn
    let delay = (6 * num_cycles) / 128;
    for _ in 0..delay {
        msp430::asm::nop()
    }
}

fn snake_leds(n: &mut u8, led_pins: &mut LEDPins) {
    *n = (*n + 1) % 4;
    match n {
        1 => led_pins.green_led.toggle().ok(),
        2 => led_pins.yellow_led.toggle().ok(),
        3 => led_pins.red_led.toggle().ok(),
        _ => Some(()),
    };
}

fn collect_payload_peripherals(cs_pins: PayloadSPIChipSelectPins) -> PayloadPeripherals {
    // Note that the peripherals gain ownership of their associated pins
    let digipot = Digipot::new(cs_pins.digipot);
    let dac = DAC::new(cs_pins.dac);
    let tether_adc = TetherADC::new(cs_pins.tether_adc);
    let temperature_adc = TemperatureADC::new(cs_pins.temperature_adc);
    let misc_adc = MiscADC::new(cs_pins.misc_adc);
    let aperture_adc = ApertureADC::new(cs_pins.aperture_adc);
    PayloadPeripherals {
        digipot,
        dac,
        tether_adc,
        temperature_adc,
        misc_adc,
        aperture_adc,
    }
}

// Takes raw port peripherals and returns actually useful pin collections
fn collect_pins(
    pmm: PMM,
    p1: P1,
    p2: P2,
    p3: P3,
    p4: P4,
    p5: P5,
    p6: P6,
) -> (
    PayloadSPIBitBangPins,
    PinpullerActivationPins,
    LEDPins,
    PayloadControlPins,
    TetherLMSPins,
    DeploySensePins,
    PayloadSPIChipSelectPins,
    DebugSerialPins,
) {
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
        sck: port4.pin5.to_output(),
    };

    let pinpuller_pins = PinpullerActivationPins {
        burn_wire_1: port3.pin2.to_output(),
        burn_wire_1_backup: port3.pin3.to_output(),
        burn_wire_2: port5.pin0.to_output(),
        burn_wire_2_backup: port5.pin1.to_output(),
    };

    let led_pins = LEDPins {
        red_led: port2.pin1.to_output(),
        yellow_led: port2.pin2.to_output(),
        green_led: port2.pin3.to_output(),
    };

    let payload_control_pins = PayloadControlPins {
        payload_enable: port6.pin6.to_output(),
        heater_enable: port4.pin4.to_output(),
        cathode_switch: port3.pin0.to_output(),
        tether_switch: port6.pin1.to_output(),
    };

    let lms_control_pins = TetherLMSPins {
        lms_receiver_enable: port3.pin4.to_output(),
        lms_led_enable: port3.pin5.to_output(),
    };

    let deploy_sense_pins = DeploySensePins {
        endmass_sense_1: port5.pin2.pulldown(),
        endmass_sense_2: port3.pin1.pulldown(),
        pinpuller_sense: port5.pin3.pullup(),
    };

    // in lieu of stateful output pins, constructor sets all pins high,
    let payload_peripheral_cs_pins = PayloadSPIChipSelectPins::new(
        port6.pin4.to_output(),
        port6.pin3.to_output(),
        port6.pin2.to_output(),
        port6.pin0.to_output(),
        port5.pin4.to_output(),
        port1.pin3.to_output(),
    );

    let debug_serial_pins = DebugSerialPins {
        rx: port4.pin2.to_output().to_alternate1(),
        tx: port4.pin3.to_output().to_alternate1(),
    };

    (
        payload_spi_pins,
        pinpuller_pins,
        led_pins,
        payload_control_pins,
        lms_control_pins,
        deploy_sense_pins,
        payload_peripheral_cs_pins,
        debug_serial_pins,
    )
}

// The compiler will emit calls to the abort() compiler intrinsic if debug assertions are
// enabled (default for dev profile). MSP430 does not actually have meaningful abort() support
// so for now, we create our own in each application where debug assertions are present.
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
