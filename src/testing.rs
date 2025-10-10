use core::convert::Infallible;

use embedded_hal::digital::v2::{InputPin, OutputPin};
use msp430fr2355::E_USCI_A1;
use msp430fr2x5x_hal::serial::{Rx, SerialUsci};
use msp430fr2x5x_hal::{gpio::Batch, pmm::Pmm};
use ufmt::{uWrite, uwrite, uwriteln};

use crate::payload::{
    HeaterState, HeaterState::*, Payload, PayloadState, PayloadState::*, SwitchState,
};
#[allow(unused_imports)]
use crate::pcb_mapping::{
    peripheral_vcc_values::*, pin_name_types::*, power_supply_limits::*, power_supply_locations::*,
    sensor_locations::*, *,
};
use crate::serial::{read_num, wait_for_any_packet, Printable, SerialWriter, TextColours::*};
#[allow(unused_imports)]
use crate::{
    adc::*,
    dac::*,
    digipot::*,
    spi::{SckPhase::SampleFirstEdge, SckPolarity::*, *},
};
use crate::{dbg_println, delay_cycles, print, println};
use fixed::{self, FixedI64};

// We use this type a lot.
/// 64 bits long, 32 fractional bits, signed.
///
/// Range: -2,147,483,648 to 2,147,483,647.
///
/// Delta: 2.3283064e-10 = 0.00000000023283064
type Fxd = FixedI64<32>;

/// Runs board diagnostics to check whether board functionality is working correctly
pub fn self_test(
    payload: Payload<{ PayloadOff }, { HeaterOff }>,
) -> Payload<{ PayloadOff }, { HeaterOff }> {
    let mut payload = payload.into_enabled_payload().into_enabled_heater();

    AutomatedFunctionalTests::full_system_test(&mut payload);
    AutomatedPerformanceTests::full_system_test(&mut payload);
    ManualFunctionalTests::full_system_test(&mut payload);

    let payload = payload.into_disabled_heater().into_disabled_payload();

    println!("Payload self test complete!");

    payload
}

/// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention.
/// These tests often rely on a sensor and an actuator together, so they test multiple components at once.
/// Functional tests are pass/fail.
pub struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests {
    pub fn full_system_test(payload: &mut Payload<{ PayloadOn }, { HeaterOn }>) {
        println!("==== Automated Functional Tests Start ====");
        for adc_test_fn in [
            Self::tether_adc_functional_test,
            Self::temperature_adc_functional_test,
            Self::misc_adc_functional_test,
            Self::aperture_adc_functional_test,
        ]
        .iter()
        {
            println!("{}", adc_test_fn(payload));
        }

        for pinpuller_lane in Self::pinpuller_functional_test(payload).iter() {
            println!("{}", pinpuller_lane);
        }

        println!("{}", Self::heater_functional_test(payload));

        for lms_channel in Self::lms_functional_test(payload).iter() {
            println!("{}", lms_channel);
        }

        println!("==== Automated Functional Tests Complete ====");
    }
    // Internal function to reduce code duplication
    fn test_adc_functional<CsPin: ADCCSPin, SENSOR: ADCSensor, const VCC: u16>(
        adc: &mut ADC<CsPin, SENSOR, VCC>,
        spi_bus: &mut impl PayloadSPI<{ IdleHigh }, { SampleFirstEdge }>,
        wanted_channel: ADCChannel,
    ) -> bool {
        let packet = (wanted_channel as u32)
            << (NUM_CYCLES_FOR_TWO_READINGS - NUM_ADDRESS_BITS - NUM_LEADING_ZEROES); // see adc.rs read_count_from
        let result = spi_bus.send_receive(NUM_CYCLES_FOR_TWO_READINGS, packet, &mut adc.cs_pin);
        let zeroes = result & 0xF000_F000;

        zeroes == 0
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    ///
    /// Dependencies: Isolated 5V supply, tether ADC, isolators
    pub fn tether_adc_functional_test<const DONTCARE: HeaterState>(
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> SensorResult<'_> {
        let result = Self::test_adc_functional(
            &mut payload.tether_adc,
            payload.spi.borrow(),
            ADCChannel::IN7,
        );
        SensorResult {
            name: "Tether ADC",
            result,
        }
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    ///
    /// Dependencies: temperature ADC
    pub fn temperature_adc_functional_test<
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
    >(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> SensorResult<'_> {
        let result = Self::test_adc_functional(
            &mut payload.temperature_adc,
            payload.spi.borrow(),
            ADCChannel::IN7,
        );
        SensorResult {
            name: "Temperature ADC",
            result,
        }
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    ///
    /// Dependencies: misc ADC
    pub fn misc_adc_functional_test<const DONTCARE1: PayloadState, const DONTCARE2: HeaterState>(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> SensorResult<'_> {
        let result =
            Self::test_adc_functional(&mut payload.misc_adc, payload.spi.borrow(), ADCChannel::IN7);
        SensorResult {
            name: "Misc ADC",
            result,
        }
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    ///
    /// Dependencies: aperture ADC, Isolated 5V supply, isolators
    pub fn aperture_adc_functional_test<
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
    >(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> SensorResult<'_> {
        payload.aperture_adc.cs_pin.set_low().ok(); // See 'payload.get_aperture_current_microamps'
        delay_cycles(5000);
        let result = Self::test_adc_functional(
            &mut payload.aperture_adc,
            payload.spi.borrow(),
            ADCChannel::IN7,
        );
        SensorResult {
            name: "Aperture ADC",
            result,
        }
    }

    /// TODO
    ///
    /// Dependencies: OBC SPI
    pub fn obc_spi_functional_test() -> bool {
        // Set interrupt on cs line(?)
        // Read spi data
        // Compare against actual value
        // Return true if recorded packet matches
        todo!();
    }

    /// Setup: Place 1.2 ohm (10W+) resistor (e.g. 30J2R0E) between pinpuller terminals
    ///
    /// Dependencies: pinpuller, pinpuller current sensor, misc ADC
    pub fn pinpuller_functional_test<
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
    >(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> [SensorResult<'_>; 4] {
        const ON_MILLIAMP_THRESHOLD: u16 = 1000; // TODO: Figure out threshhold
        let mut results = [false; 4];

        // Enable each of the four redundant lines.
        // Measure current
        // Return success if current above X mA

        for n in 0..4 {
            pin_select(payload, n).0.set_high().ok();
            results[n] = payload.get_pinpuller_current_milliamps() > ON_MILLIAMP_THRESHOLD;
            pin_select(payload, n).0.set_low().ok();
            delay_cycles(1000);
        }

        [
            SensorResult {
                name: "Pinpuller channel 1",
                result: results[0],
            },
            SensorResult {
                name: "Pinpuller channel 1 backup",
                result: results[1],
            },
            SensorResult {
                name: "Pinpuller channel 2",
                result: results[2],
            },
            SensorResult {
                name: "Pinpuller channel 2 backup",
                result: results[3],
            },
        ]
    }

    /// Set the heater to the minimum, wait 0.1s and read voltage. Set to maximum, wait 0.1s, read voltage.
    /// Check these values are within 10% of expected values.
    ///
    /// Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn heater_functional_test<'a>(
        payload: &mut Payload<{ PayloadOn }, { HeaterOn }>,
    ) -> SensorResult<'a> {
        // Set heater to min
        payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS); // set voltage
        delay_cycles(100_000);
        // Read voltage
        let min_voltage_mv = payload.get_heater_voltage_millivolts();
        dbg_println!(
            "Min voltage set to {}. Read as {}, expected at most {}",
            HEATER_MIN_VOLTAGE_MILLIVOLTS,
            min_voltage_mv,
            (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32) * 11 / 10
        );

        // Set heater to max
        payload.set_heater_voltage(HEATER_MAX_VOLTAGE_MILLIVOLTS); // set voltage
        delay_cycles(100_000);
        // Read voltage
        let max_voltage_mv = payload.get_heater_voltage_millivolts();
        dbg_println!(
            "Max voltage set to {}. Read as {}, expected at least {}",
            HEATER_MAX_VOLTAGE_MILLIVOLTS,
            max_voltage_mv,
            (HEATER_MAX_VOLTAGE_MILLIVOLTS as u32) * 9 / 10
        );

        // Set heater back to min and give time to settle
        payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS); // set voltage
        delay_cycles(1_000_000);

        SensorResult {
            name: "Heater",
            result: ((min_voltage_mv as u32) < (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32) * 11 / 10)
                && ((max_voltage_mv as u32) > (HEATER_MAX_VOLTAGE_MILLIVOLTS as u32) * 9 / 10),
        }
    }

    /// Enable receivers, record ambient values. Enable LEDs, record values. Return ok if on_value > 2 * ambient.
    ///
    /// Setup: Connect LMS board, test in a room with minimal (or at least uniform) IR interference.
    /// Dependencies: LMS power switches, misc ADC, LMS LEDs, LMS receivers
    pub fn lms_functional_test<const DONTCARE1: PayloadState, const DONTCARE2: HeaterState>(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> [SensorResult<'_>; 3] {
        let mut ambient_counts: [u16; 3] = [0; 3];
        let mut on_counts: [u16; 3] = [0; 3];

        // Enable phototransistors
        payload.lms_control_pins.lms_led_enable.set_low().ok();
        payload.lms_control_pins.lms_receiver_enable.set_high().ok();
        delay_cycles(100_000);

        // Record max voltage/light value
        for (n, sensor) in [
            LMS_RECEIVER_1_SENSOR,
            LMS_RECEIVER_2_SENSOR,
            LMS_RECEIVER_3_SENSOR,
        ]
        .iter()
        .enumerate()
        {
            ambient_counts[n] = payload
                .misc_adc
                .read_count_from(sensor, payload.spi.borrow());
        }
        dbg_println!("Read ambient counts as: {:?}", ambient_counts);

        // Enable LEDs
        payload.lms_control_pins.lms_led_enable.set_high().ok();
        delay_cycles(100_000);

        // Record max voltage/light value
        for (n, sensor) in [
            LMS_RECEIVER_1_SENSOR,
            LMS_RECEIVER_2_SENSOR,
            LMS_RECEIVER_3_SENSOR,
        ]
        .iter()
        .enumerate()
        {
            on_counts[n] = payload
                .misc_adc
                .read_count_from(sensor, payload.spi.borrow());
        }
        dbg_println!("Read max counts as: {:?}", on_counts);

        payload.lms_control_pins.lms_receiver_enable.set_low().ok();
        payload.lms_control_pins.lms_led_enable.set_low().ok();

        [
            SensorResult {
                name: "Length measurement system 1",
                result: (on_counts[0] > 2 * ambient_counts[0]),
            },
            SensorResult {
                name: "Length measurement system 2",
                result: (on_counts[1] > 2 * ambient_counts[1]),
            },
            SensorResult {
                name: "Length measurement system 3",
                result: (on_counts[2] > 2 * ambient_counts[2]),
            },
        ]
    }
}

/// Selects one of the four pinpuller lines based on an integer value
fn pin_select<'a, 'b: 'a, const DC1: PayloadState, const DC2: HeaterState>(
    payload: &'b mut Payload<DC1, DC2>,
    n: usize,
) -> (&'a mut dyn OutputPin<Error = void::Void>, &'static str) {
    match n {
        0 => (&mut payload.pinpuller_pins.burn_wire_1, "Burn Wire 1"),
        1 => (
            &mut payload.pinpuller_pins.burn_wire_1_backup,
            "Burn Wire 1 backup",
        ),
        2 => (&mut payload.pinpuller_pins.burn_wire_2, "Burn Wire 2"),
        _ => (
            &mut payload.pinpuller_pins.burn_wire_2_backup,
            "Burn Wire 2 backup",
        ),
    }
}

/// Rather than using percent error (which isn't defined when the actual value is zero), we use Relative Percent Difference (RPD).
/// Outputs are between -1 and 1. Values near zero are close to percentage error, but 1 means measured is infinitely larger than actual, -1 means measured is infinitely smaller than actual.
pub fn calculate_rpd(measured: i32, actual: i32) -> Fxd {
    if actual == 0 && measured == 0 {
        return Fxd::ZERO;
    }
    let actual = Fxd::from(actual);
    let measured = Fxd::from(measured);

    // (measured - actual) / measured.abs() + actual.abs()
    (measured - actual)
        .checked_div(measured.abs() + actual.abs())
        .unwrap_or_else(|| {
            if measured - actual > 0 {
                Fxd::MAX
            } else {
                Fxd::MIN
            }
        }) // This only fires on overflow.
           // Ideally we could use saturating_div instead of checked_div and unwrap_or_else
           // but the panic_never condition fails even if we check denominator is non-zero first...
}
/// Iteratively updates an average with a new value
pub fn in_place_average(acc: Fxd, new: Fxd, n: u16) -> Fxd {
    //acc + ((new - acc) / Fxd::from(n+1))
    acc + ((new - acc)
        .checked_div(Fxd::from(n + 1))
        .unwrap_or(Fxd::ZERO)) // unwrap_or should never fire, since n+1 > 0 when n is unsigned.
}

pub fn calculate_performance_result(
    name: &str,
    rpd: Fxd,
    succ_percent: u8,
    inacc_percent: u8,
) -> PerformanceResult<'_> {
    let performance = match rpd.abs() {
        x if x < Fxd::from(succ_percent) / 100 => Performance::Nominal,
        x if x < Fxd::from(inacc_percent) / 100 => Performance::Inaccurate,
        _ => Performance::NotWorking,
    };
    PerformanceResult {
        name,
        performance,
        accuracy: rpd,
    }
}

/// Calculates a square root approximation using Newton's method. Panics on negative values.
const fn fixed_sqrt(x: Fxd) -> Fxd {
    if x.is_negative() {
        panic!();
    } else if x.is_zero() {
        return Fxd::ZERO;
    }
    let mut guess = x;

    // No for loops in const fn's yet.
    let mut iterations = 10;
    while iterations > 0 {
        guess = (guess.unwrapped_add(x.unwrapped_div(guess))).unwrapped_div_int(2);
        iterations -= 1;
    }
    return guess;
}

/// Accuracy-based tests that can be run automatically, possibly after some initial setup.
pub struct AutomatedPerformanceTests {}
impl AutomatedPerformanceTests {
    pub fn full_system_test(payload: &mut Payload<{ PayloadOn }, { HeaterOn }>) {
        println!("==== Automatic Performance Tests Start ====");
        // Each of these three fn's takes the same arguments and both return a voltage and current result
        let fn_arr = [
            Self::test_cathode_offset,
            Self::test_tether_bias,
            Self::test_heater,
        ];
        for sensor_fn in fn_arr.iter() {
            for sensor_result in sensor_fn(payload).iter() {
                println!("{}", sensor_result);
            }
        }
        println!("{}", Self::test_pinpuller_current_sensor(payload));

        println!("==== Automatic Performance Tests Complete ====\n");
    }
    pub fn full_system_emitter_test(payload: &mut Payload<{ PayloadOn }, { HeaterOn }>) {
        println!("==== Automatic Emitter Performance Tests Start ====");
        // Each of these three fn's takes the same arguments and both return a voltage and current result
        let fn_arr = [
            Self::test_cathode_offset_voltage,
            Self::test_tether_bias_voltage,
        ];
        for sensor_fn in fn_arr.iter() {
            println!("{}", sensor_fn(payload));
        }

        for sensor_result in Self::test_heater(payload).iter() {
            println!("{}", sensor_result);
        }
        println!("{}", Self::test_pinpuller_current_sensor(payload));

        println!("==== Automatic Performance Tests Complete ====\n");
    }
    /// Setup: Place a 100k resistor between exterior and cathode-
    ///
    /// Dependencies: Isolated 5V supply, tether ADC, DAC, cathode offset supply, signal processing circuitry, isolators
    pub fn test_cathode_offset<const DONTCARE: HeaterState>(
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> [PerformanceResult<'_>; 2] {
        let [voltage_accuracy, current_accuracy] = Self::test_hvdc_supply(
            &Payload::set_cathode_offset_switch,
            &Payload::get_cathode_offset_voltage_millivolts,
            &Payload::get_cathode_offset_current_microamps,
            &Payload::set_cathode_offset_voltage,
            CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS,
            CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS,
            hvdc_mock::MOCK_CATHODE_OFFSET_RESISTANCE_OHMS,
            payload,
        );

        let voltage_result =
            calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        let current_result =
            calculate_performance_result("Cathode offset current", current_accuracy, 5, 20);
        [voltage_result, current_result]
    }

    pub fn test_cathode_offset_voltage<const DONTCARE: HeaterState>(
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> PerformanceResult<'_> {
        let voltage_accuracy = Self::test_hvdc_supply_voltage(
            &Payload::set_cathode_offset_switch,
            &Payload::get_cathode_offset_voltage_millivolts,
            &Payload::set_cathode_offset_voltage,
            CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS,
            200_000,
            payload,
        );

        let voltage_result =
            calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    /// Setup: Place a 100k resistor between tether and cathode-
    ///
    /// Dependencies: isolated 5V supply, tether ADC, DAC, tether bias supply, signal processing circuitry, isolators
    pub fn test_tether_bias<const DONTCARE: HeaterState>(
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> [PerformanceResult<'_>; 2] {
        let [voltage_accuracy, current_accuracy] = Self::test_hvdc_supply(
            &Payload::set_tether_bias_switch,
            &Payload::get_tether_bias_voltage_millivolts,
            &Payload::get_tether_bias_current_microamps,
            &Payload::set_tether_bias_voltage,
            TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS,
            TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS,
            hvdc_mock::MOCK_TETHER_BIAS_RESISTANCE_OHMS,
            payload,
        );

        let voltage_result =
            calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
        let current_result =
            calculate_performance_result("Tether bias current", current_accuracy, 5, 20);
        [voltage_result, current_result]
    }

    pub fn test_tether_bias_voltage<const DONTCARE: HeaterState>(
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> PerformanceResult<'_> {
        let voltage_accuracy = Self::test_hvdc_supply_voltage(
            &Payload::set_tether_bias_switch,
            &Payload::get_tether_bias_voltage_millivolts,
            &Payload::set_tether_bias_voltage,
            TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS,
            200_000,
            payload,
        );

        let voltage_result =
            calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    /// Internal function to reduce code duplication.
    fn test_hvdc_supply<const DONTCARE: HeaterState>(
        set_switch_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>, SwitchState),
        measure_voltage_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>) -> i32,
        measure_current_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>) -> i32,
        set_voltage_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>, u32),
        supply_min: u32,
        supply_max: u32,
        test_resistance: u32,
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> [Fxd; 2] {
        const NUM_MEASUREMENTS: usize = 10;
        const SENSE_RESISTANCE: u32 = 1; // Both supplies use the same sense resistor value
        const TEST_START_PERCENT: u32 = 10;
        const TEST_END_PERCENT: u32 = 100;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;
        let mut current_accuracy: Fxd = Fxd::ZERO;

        set_switch_fn(payload, SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (TEST_START_PERCENT..=TEST_END_PERCENT)
            .step_by(100 / NUM_MEASUREMENTS)
            .enumerate()
        {
            let set_voltage_mv: u32 =
                ((100 - output_percentage) * (supply_min) + output_percentage * (supply_max)) / 100;
            dbg_println!("");
            dbg_println!("Target output voltage: {}mV", set_voltage_mv);

            // Set cathode voltage
            set_voltage_fn(payload, set_voltage_mv);
            dbg_println!("Set target voltage");

            delay_cycles(100_000); //settling time

            // Read voltage, current
            let measured_voltage_mv = measure_voltage_fn(payload);
            let measured_current_ua = measure_current_fn(payload);
            dbg_println!("Measured output voltage: {}mV", measured_voltage_mv);
            dbg_println!("Measured output current: {}uA", measured_current_ua);

            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = set_voltage_mv as i32;
            let expected_current_ua: i32 =
                ((1000 * set_voltage_mv) / (test_resistance + SENSE_RESISTANCE)) as i32;

            dbg_println!("Expected output voltage: {}mV", expected_voltage_mv);
            dbg_println!("Expected output current: {}uA", expected_current_ua);

            let voltage_rpd = calculate_rpd(measured_voltage_mv, expected_voltage_mv);
            let current_rpd = calculate_rpd(measured_current_ua, expected_current_ua);

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
            current_accuracy = in_place_average(current_accuracy, current_rpd, i as u16);
        }

        // Set back to zero
        set_voltage_fn(payload, (supply_min + supply_max) / 100);

        set_switch_fn(payload, SwitchState::Disconnected);

        [voltage_accuracy, current_accuracy]
    }

    /// Internal function to reduce code duplication.
    fn test_hvdc_supply_voltage<const DONTCARE: HeaterState>(
        set_switch_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>, SwitchState),
        measure_voltage_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>) -> i32,
        set_voltage_fn: &dyn Fn(&mut Payload<{ PayloadOn }, DONTCARE>, u32),
        supply_min: u32,
        supply_max: u32,
        payload: &mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> Fxd {
        const NUM_MEASUREMENTS: usize = 10;
        const SENSE_RESISTANCE: u32 = 1; // Both supplies use the same sense resistor value
        const TEST_START_PERCENT: u32 = 10;
        const TEST_END_PERCENT: u32 = 100;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        set_switch_fn(payload, SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (TEST_START_PERCENT..=TEST_END_PERCENT)
            .step_by(100 / NUM_MEASUREMENTS)
            .enumerate()
        {
            let set_voltage_mv: u32 =
                ((100 - output_percentage) * (supply_min) + output_percentage * (supply_max)) / 100;
            dbg_println!("Target output voltage: {}mV", set_voltage_mv);

            // Set cathode voltage
            set_voltage_fn(payload, set_voltage_mv);
            dbg_println!("Set target voltage");

            delay_cycles(100_000); //settling time

            // Read voltage, current
            let measured_voltage_mv = measure_voltage_fn(payload);
            dbg_println!("Measured output voltage: {}mV", measured_voltage_mv);

            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = set_voltage_mv as i32;
            dbg_println!("Expected output voltage: {}mV", expected_voltage_mv);

            let voltage_rpd = calculate_rpd(measured_voltage_mv, expected_voltage_mv);

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
            dbg_println!("");
        }

        // Set back to zero
        set_voltage_fn(payload, (supply_min + supply_max) / 100);

        set_switch_fn(payload, SwitchState::Disconnected);

        voltage_accuracy
    }
    /// Setup: 10 ohm resistor across heater+ and heater-
    ///
    /// Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn test_heater(
        payload: &mut Payload<{ PayloadOn }, { HeaterOn }>,
    ) -> [PerformanceResult<'_>; 2] {
        const NUM_MEASUREMENTS: usize = 10;

        let mut voltage_accuracy: Fxd = Fxd::ZERO;
        let mut current_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (0..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = (((100 - output_percentage)
                * (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32)
                + output_percentage * (2_000 as u32))
                / 100) as u16;

            // Set cathode voltage
            payload.set_heater_voltage(output_voltage_mv);

            dbg_println!("");
            dbg_println!("Set voltage to: {}mV", output_voltage_mv);
            delay_cycles(100_000); //settling time

            // Read voltage, current
            let heater_voltage_mv = payload.get_heater_voltage_millivolts();
            dbg_println!("Read voltage as: {}mV", heater_voltage_mv);
            let heater_current_ma = payload.get_heater_current_milliamps();
            dbg_println!("Read current as: {}mA", heater_current_ma);

            // Calculate expected voltage and current
            let expected_voltage_mv: u16 = output_voltage_mv;
            let expected_current_ma: i16 =
                (expected_voltage_mv as u32 * 1000 / heater_mock::CIRCUIT_RESISTANCE_MOHMS as u32)
                    .min(heater_mock::POWER_LIMITED_MAX_CURRENT_MA.to_num()) as i16;
            dbg_println!("Expected current is: {}mA", expected_current_ma);

            let voltage_rpd = calculate_rpd(heater_voltage_mv as i32, expected_voltage_mv as i32);
            dbg_println!(
                "Voltage milliRPD is: {}",
                (voltage_rpd * 1000).to_num::<i32>()
            );
            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
            current_accuracy = in_place_average(
                current_accuracy,
                calculate_rpd(heater_current_ma as i32, expected_current_ma as i32),
                i as u16,
            );
        }

        let voltage_result =
            calculate_performance_result("Heater voltage", voltage_accuracy, 5, 20);
        let current_result =
            calculate_performance_result("Heater current", current_accuracy, 5, 20);
        [voltage_result, current_result]
    }

    /// Setup: Place 1.2 ohm (10W+) resistor between pinpuller pins.
    ///
    /// Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry
    pub fn test_pinpuller_current_sensor<
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
    >(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> PerformanceResult<'_> {
        let mut accuracy: Fxd = Fxd::ZERO;

        // For each pin, activate the pinpuller through that channel and measure the current
        dbg_println!("");
        for n in 0..4 {
            pin_select(payload, n).0.set_high().ok();
            delay_cycles(1_000);
            let measured_current = payload.get_pinpuller_current_milliamps();
            dbg_println!("Measured current as {}mA", measured_current);
            accuracy = in_place_average(
                accuracy,
                calculate_rpd(
                    measured_current as i32,
                    pinpuller_mock::EXPECTED_ON_CURRENT.to_num(),
                ),
                n as u16,
            );
            pin_select(payload, n).0.set_low().ok();
            delay_cycles(1000);
        }

        calculate_performance_result("Pinpuller current sense", accuracy, 5, 20)
    }

    // Connect repeller plate to HVDC tether supply (Pin 3 of S1_TBS) to cover a range of 25-250V
    pub fn test_repeller_voltage<'a, const DONTCARE: HeaterState, USCI: SerialUsci>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
        spi_bus: &'a mut PayloadSPIController,
        debug_writer: &mut SerialWriter<USCI>,
    ) -> [PerformanceResult<'a>; 1] {
        let mut voltage_accuracy: Fxd = Fxd::ZERO;
        let supply_min: u32 = TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS;
        let supply_max: u32 = TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS;
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_START_PERCENT: u32 = 10;
        const TEST_END_PERCENT: u32 = 100;

        Payload::set_tether_bias_switch(payload, SwitchState::Connected);

        for (i, output_percentage) in (TEST_START_PERCENT..=TEST_END_PERCENT)
            .step_by(100 / NUM_MEASUREMENTS)
            .enumerate()
        {
            // Set tether voltage
            let set_voltage_mv: u32 =
                ((100 - output_percentage) * (supply_min) + output_percentage * (supply_max)) / 100;
            payload.set_tether_bias_voltage(set_voltage_mv);
            dbg_println!("Target output voltage: {}mV", set_voltage_mv);
            delay_cycles(100_000); //settling time

            // Measure repeller voltage (with this config, this should be the same as the tether bias voltage measurement)
            let measured_repeller_voltage_mv = payload.get_repeller_voltage_millivolts();
            let measured_tether_voltage_mv = payload.get_tether_bias_voltage_millivolts();
            dbg_println!(
                "Measured repeller voltage: {}mV",
                measured_repeller_voltage_mv
            );
            dbg_println!("Measured tether voltage: {}mV", measured_tether_voltage_mv);

            // Measure rpd and accuracy
            let voltage_rpd =
                calculate_rpd(measured_repeller_voltage_mv as i32, set_voltage_mv as i32);
            dbg_println!(
                "Voltage milliRPD is: {}",
                (voltage_rpd * 1000).to_num::<i32>()
            );
            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
        }

        Payload::set_tether_bias_switch(payload, SwitchState::Disconnected);

        let voltage_result =
            calculate_performance_result("Repeller voltage", voltage_accuracy, 5, 20);
        [voltage_result]
    }

    pub fn test_aperture_current_sensor<USCI: SerialUsci>(
        payload: &mut Payload<{ PayloadOn }, { HeaterOn }>,
        spi_bus: &mut PayloadSPIController,
        serial_writer: &mut SerialWriter<USCI>,
    ) {
        uwriteln!(serial_writer, "Here1").ok();
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS);
        uwriteln!(serial_writer, "Here2").ok();
        payload.set_cathode_offset_switch(SwitchState::Connected);
        uwriteln!(serial_writer, "Here3").ok();
        payload.set_tether_bias_voltage(TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS);
        uwriteln!(serial_writer, "Here4").ok();
        payload.set_tether_bias_switch(SwitchState::Disconnected);
        uwriteln!(serial_writer, "Here5").ok();

        for cycles in 1..4 {
            for heater_voltage_mv in (900..3100).step_by(100) {
                uwriteln!(
                    serial_writer,
                    "Heater voltage set to: {}mV",
                    heater_voltage_mv
                )
                .ok();
                payload.set_heater_voltage(heater_voltage_mv);
                delay_cycles(1_000_000);

                let measured_heater_voltage_mv = payload.get_heater_voltage_millivolts();
                let measured_cathode_offset_voltage_mv =
                    payload.get_cathode_offset_voltage_millivolts();
                let measured_cathode_offset_current_ua =
                    payload.get_cathode_offset_current_microamps();
                let measured_aperture_adc_mv = payload
                    .aperture_adc
                    .read_voltage_from(&APERTURE_CURRENT_SENSOR, &mut payload.spi);
                let measured_aperture_current_ua =
                    self::sensor_equations::aperture_current_sensor_eq(measured_aperture_adc_mv);

                uwriteln!(
                    serial_writer,
                    "Measured heater voltage: {}mV",
                    measured_heater_voltage_mv
                )
                .ok();
                uwriteln!(
                    serial_writer,
                    "Measured cathode offset voltage: {}mV",
                    measured_cathode_offset_voltage_mv
                )
                .ok();
                uwriteln!(
                    serial_writer,
                    "Measured cathode offset current: {}uA",
                    measured_cathode_offset_current_ua
                )
                .ok();
                uwriteln!(
                    serial_writer,
                    "Measured aperture ADC voltage: {}mV",
                    measured_aperture_adc_mv
                )
                .ok();
                uwriteln!(
                    serial_writer,
                    "Measured aperture current: {}uA",
                    measured_aperture_current_ua
                )
                .ok();
                uwriteln!(serial_writer, "").ok();
                delay_cycles(3_000_000);
            }
        }
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS);
        payload.set_cathode_offset_switch(SwitchState::Disconnected);
    }
}

/// Tests that require human intervention. These are pass/fail tests.
pub struct ManualFunctionalTests {}
impl ManualFunctionalTests {
    pub fn full_system_test<const DONTCARE1: PayloadState, const DONTCARE2: HeaterState>(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) {
        println!("==== Manual Functional Tests Start ====");

        for result in Self::endmass_switches_functional_test(payload).iter() {
            println!("{}", result);
        }

        println!("==== Manual Functional Tests Complete ====");
    }
    /// Dependencies: endmass switches
    pub fn endmass_switches_functional_test<
        'b,
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
    >(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
    ) -> [SensorResult<'b>; 2] {
        println!("Depress switches then press enter");
        wait_for_any_packet(&mut payload.serial_reader);

        // Note: is_low/is_high is infallible, so ignore the unwraps
        let is_depressed_arr: [bool; 2] = [
            payload
                .deploy_sense_pins
                .endmass_sense_1
                .is_low()
                .unwrap_or(false),
            payload
                .deploy_sense_pins
                .endmass_sense_2
                .is_low()
                .unwrap_or(false),
        ];

        println!("Release switches then press enter");
        wait_for_any_packet(&mut payload.serial_reader);

        let is_released_arr: [bool; 2] = [
            payload
                .deploy_sense_pins
                .endmass_sense_1
                .is_high()
                .unwrap_or(false),
            payload
                .deploy_sense_pins
                .endmass_sense_2
                .is_high()
                .unwrap_or(false),
        ];

        [
            SensorResult {
                name: "Endmass switch 1",
                result: (is_depressed_arr[0] && is_released_arr[0]),
            },
            SensorResult {
                name: "Endmass switch 2",
                result: (is_depressed_arr[1] && is_released_arr[1]),
            },
        ]
    }

    // Dependencies: pinpuller
    pub fn pinpuller_functional_test<
        'a,
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
        USCI: SerialUsci,
    >(
        payload: &mut Payload<DONTCARE1, DONTCARE2>,
        serial_reader: &mut Rx<USCI>,
    ) -> [PerformanceResult<'a>; 4] {
        // Enable each of the four redundant lines.

        let mut result: [PerformanceResult; 4] = Default::default();
        // Manually check resistance(?) across pinpuller pins
        for n in 0..4 {
            let (pin, name) = pin_select(payload, n);
            pin.set_high().ok();
            let measured = read_num(serial_reader);
            println!("{} active.", name);
            println!("Please enter current:");
            pin.set_low().ok();
            delay_cycles(1000);
            result[n] = calculate_performance_result(
                name,
                calculate_rpd(measured, pinpuller_mock::EXPECTED_ON_CURRENT.to_num()),
                5,
                20,
            );
        }
        result
    }
    /*
    // Dependencies: LMS power switches
    pub fn lms_power_switch_functional_test() -> [SensorResult; 2] {
        // Enable LMS LED EN
        // Measure resistance between J1 pin 2A/B and GND
        // Enable LMS Receiver EN
        // Manually measure resistance between J1 pin 3A/B and GND
        // Query user for resistance
        // Return true if resistance less than 1-10 ohms
        todo!();
    }*/
}

/// Values associated with mock pinpuller tests
pub mod pinpuller_mock {
    use super::{Fxd, PINPULLER_VOLTAGE_MILLIVOLTS};

    const MOSFET_R_ON_RESISTANCE: Fxd = Fxd::lit("0.03"); // Verify(?)
    const PINPULLER_MOCK_RESISTANCE: Fxd = Fxd::lit("1.2");
    const DIVIDER_RESISTANCE: Fxd = Fxd::lit("0.4");
    const SENSE_RESISTANCE: Fxd = Fxd::lit("0.082");
    const WIRE_RESISTANCE: Fxd = Fxd::lit("0.22");
    const CIRCUIT_RESISTANCE: Fxd = WIRE_RESISTANCE
        .unwrapped_add(SENSE_RESISTANCE)
        .unwrapped_add(DIVIDER_RESISTANCE)
        .unwrapped_add(PINPULLER_MOCK_RESISTANCE)
        .unwrapped_add(MOSFET_R_ON_RESISTANCE)
        .unwrapped_add(MOSFET_R_ON_RESISTANCE);
    const NUM_PINS: usize = 4;
    pub const EXPECTED_ON_CURRENT: Fxd =
        Fxd::const_from_int(PINPULLER_VOLTAGE_MILLIVOLTS as i64).unwrapped_div(CIRCUIT_RESISTANCE);
}

/// Values associated with mock heater tests
pub mod heater_mock {
    use super::{fixed_sqrt, Fxd};

    const MOCK_HEATER_RESISTANCE_MOHMS: u16 = 10_000;
    const PROBE_RESISTANCE_MOHMS: u16 = 90;
    pub const CIRCUIT_RESISTANCE_MOHMS: u16 =
        MOCK_HEATER_RESISTANCE_MOHMS + super::HEATER_SENSE_RESISTANCE_MILLIOHMS as u16; // heater resistance + shunt resistor
    pub const CIRCUIT_AND_PROBE_RESISTANCE_MOHMS: u16 =
        CIRCUIT_RESISTANCE_MOHMS + PROBE_RESISTANCE_MOHMS;
    const HEATER_MAX_POWER_MWATTS: u16 = 1000; // TODO: Verify?

    pub const POWER_LIMITED_MAX_CURRENT_MA: Fxd = fixed_sqrt(
        Fxd::const_from_int(HEATER_MAX_POWER_MWATTS as i64)
            .unwrapped_div_int(CIRCUIT_RESISTANCE_MOHMS as i64),
    )
    .unwrapped_mul_int(1000); //sqrt(heater_max_power_mw / circuit_resistance_mohm) * 1000;
}

pub mod hvdc_mock {
    pub const MOCK_TETHER_BIAS_RESISTANCE_OHMS: u32 = 98_150;
    pub const MOCK_CATHODE_OFFSET_RESISTANCE_OHMS: u32 = 98_300;
}

fn test_temperature_sensors_against_known_temp<
    'a,
    const DONTCARE1: PayloadState,
    const DONTCARE2: HeaterState,
    USCI: SerialUsci,
>(
    room_temp_k: u16,
    payload: &'a mut Payload<DONTCARE1, DONTCARE2>,
    serial_writer: &'a mut SerialWriter<USCI>,
    serial_reader: &'a mut Rx<USCI>,
    spi_bus: &'a mut PayloadSPIController,
) -> [PerformanceResult<'static>; 8] {
    const TEMP_SENSORS: [(TemperatureSensor, &str); 8] = [
        (LMS_EMITTER_TEMPERATURE_SENSOR, "LMS Emitter"),
        (LMS_RECEIVER_TEMPERATURE_SENSOR, "LMS Receiver"),
        (MSP430_TEMPERATURE_SENSOR, "MSP430"),
        (HEATER_SUPPLY_TEMPERATURE_SENSOR, "Heater supply"),
        (HVDC_SUPPLIES_TEMPERATURE_SENSOR, "HVDC Supplies"),
        (TETHER_MONITORING_TEMPERATURE_SENSOR, "Tether monitoring"),
        (TETHER_CONNECTOR_TEMPERATURE_SENSOR, "Tether connector"),
        (MSP_3V3_TEMPERATURE_SENSOR, "MSP 3V3 supply"),
    ];

    let mut output_arr: [PerformanceResult; 8] = [PerformanceResult::default(); 8];
    for (n, (sensor, name)) in TEMP_SENSORS.iter().enumerate() {
        let tempr = payload.get_temperature_kelvin(sensor);
        let accuracy = calculate_rpd(tempr as i32, room_temp_k as i32);
        output_arr[n] = calculate_performance_result(name, accuracy, 5, 20)
    }

    output_arr
}

const CELCIUS_TO_KELVIN_OFFSET: u16 = 273;
// Accuracy-based tests
pub struct ManualPerformanceTests {}
impl ManualPerformanceTests {
    /*
    // Dependencies: Isolated 5V supply, Tether ADC, isolators
    pub fn test_tether_adc() -> PerformanceResult {
        // Manually apply known voltage to channel.
        // Ask to read channel X.
        // Return success if SPI packet valid and accuracy within 10%
        todo!();
    }
    // Dependencies: Temperature ADC
    pub fn test_temperature_adc() -> PerformanceResult {
        // Manually apply known voltage to channel.
        // Ask to read channel X.
        // Return success if SPI packet valid and accuracy within 10%
        todo!();
    }
    // Dependencies: Misc ADC
    pub fn test_misc_adc() -> PerformanceResult {
        // Manually apply known voltage to channel.
        // Ask to read channel X.
        // Return success if SPI packet valid and accuracy within 10%
        todo!();
    }*/
    /// Get room temp from user
    fn query_room_temp<USCI: SerialUsci>(
        serial_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI>,
    ) -> u16 {
        println!("Enter current temp (in celcius)");
        let celcius_num = read_num(serial_reader);
        (celcius_num + CELCIUS_TO_KELVIN_OFFSET as i32) as u16
    }
    pub fn two_point_test_temperature_sensor_test<
        'a,
        USCI: SerialUsci,
        const DONTCARE: HeaterState,
    >(
        payload: &'a mut Payload<{ PayloadOff }, DONTCARE>, // Minimise heat generation
        serial_writer: &'a mut SerialWriter<USCI>,
        serial_reader: &'a mut Rx<USCI>,
        spi_bus: &'a mut PayloadSPIController,
    ) -> [PerformanceResult<'a>; 8] {
        let mut room_temp_k: u16 = Self::query_room_temp(serial_writer, serial_reader);

        let arr1 = test_temperature_sensors_against_known_temp(
            room_temp_k,
            payload,
            serial_writer,
            serial_reader,
            spi_bus,
        );

        room_temp_k = Self::query_room_temp(serial_writer, serial_reader);

        let arr2 = test_temperature_sensors_against_known_temp(
            room_temp_k,
            payload,
            serial_writer,
            serial_reader,
            spi_bus,
        );

        let mut result_arr: [PerformanceResult; 8] = [PerformanceResult::default(); 8];

        for (n, (result1, result2)) in arr1.iter().zip(arr2.iter()).enumerate() {
            let accuracy = (result1.accuracy + result2.accuracy) / 2;
            result_arr[n] = calculate_performance_result(result1.name, accuracy, 5, 20)
        }
        result_arr
    }

    /// Dependencies: Isolated 5V supply, DAC, isolators
    pub fn test_dac<'a, const DONTCARE: HeaterState, USCI: SerialUsci>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
        spi_bus: &'a mut impl PayloadSPI<{ IdleLow }, { SampleFirstEdge }>,
        debug_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 5;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (1..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 =
                ((output_percentage * DAC_VCC_VOLTAGE_MILLIVOLTS as u32) / 100) as u16;
            let dac_count = DAC::voltage_to_count(output_voltage_mv);
            uwriteln!(
                debug_writer,
                "Target output voltage: {}mV. DAC count: {}",
                output_voltage_mv,
                dac_count
            )
            .ok();

            // Set DAC voltage
            payload.dac.send_command(
                DACCommand::WriteToAndUpdateRegisterX,
                DACChannel::ChannelC,
                dac_count,
                spi_bus,
            );

            delay_cycles(1000); //settling time

            // Read cathode voltage, current
            uwrite!(debug_writer, "Measure voltage and enter in mV: ").ok();
            let measured_voltage_mv = read_num(serial_reader);
            uwriteln!(debug_writer, "").ok();

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            uwriteln!(
                debug_writer,
                "Calculated voltage millirpd: {}",
                (voltage_rpd * 1000).to_num::<i32>()
            )
            .ok();

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
        }

        // Set back to zero
        payload.dac.send_command(
            DACCommand::WriteToAndUpdateRegisterX,
            DACChannel::ChannelA,
            DAC::voltage_to_count(0),
            spi_bus,
        );

        let voltage_result =
            calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        voltage_result
    }
    /*
    // Dependencies: Isolated 5V supply, digipot, isolators
    pub fn test_digipot() -> PerformanceResult {
        // Set channel X
        // Measure resistance by hand.
        // Query user about resistance
        // Return success if resistance within 10%
        todo!();
    }
    // Dependencies: (???), misc_adc
    pub fn test_aperture() -> PerformanceResult {
        // Manudally generate aperture current (setup TBD)
        // Measure current
        // Calculate expected current
        // Return success if
        todo!();
    }*/

    /// Dependencies: DAC
    pub fn test_cathode_offset_voltage<'a, const DONTCARE: HeaterState>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
        // spi_bus: &'a mut PayloadSPIController,
        // debug_writer: &mut SerialWriter<USCI>,
        // serial_reader: &mut Rx<USCI>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        payload.set_cathode_offset_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (10..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100 - output_percentage)
                * (CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS)
                + output_percentage * (CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS))
                / 100;
            println!(
                "Target output voltage: {}mV",
                output_voltage_mv
            );

            // Set cathode voltage
            payload.set_cathode_offset_voltage(output_voltage_mv);

            delay_cycles(10000); //settling time

            // Read cathode voltage, current
            print!("Measure voltage and input (in mV): ");
            let measured_voltage_mv = read_num(&mut payload.serial_reader);
            println!("");

            println!(
                "Cathode offset mv: {}",
                payload.get_cathode_offset_voltage_millivolts()
            );

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            println!(
                "Calculated voltage millirpd: {}",
                (voltage_rpd * 1000).to_num::<i32>()
            );

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
        }

        // Set back to zero
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS);
        payload.set_cathode_offset_switch(SwitchState::Disconnected);

        let voltage_result =
            calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    pub fn test_repeller_voltage<'a, const DONTCARE: HeaterState>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
        // spi_bus: &'a mut PayloadSPIController,
        // debug_writer: &mut SerialWriter<USCI>,
        // serial_reader: &mut Rx<USCI>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        // payload.set_cathode_offset_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (10..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100 - output_percentage)
                * (REPELLER_MIN_VOLTAGE_MILLIVOLTS)
                + output_percentage * (REPELLER_MAX_VOLTAGE_MILLIVOLTS))
                / 100;
            println!(
                "Target output voltage: {}mV",
                output_voltage_mv
            );

            // Set cathode voltage
            // payload. (output_voltage_mv);

            delay_cycles(10000); //settling time

            // Read cathode voltage, current
            print!("Measure voltage and input (in mV): ");
            let measured_voltage_mv = read_num(&mut payload.serial_reader);
            println!("");

            println!(
                "Cathode offset mv: {}",
                payload.get_cathode_offset_voltage_millivolts()
            );

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            println!(
                "Calculated voltage millirpd: {}",
                (voltage_rpd * 1000).to_num::<i32>()
            );

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
        }

        // Set back to zero
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS);
        payload.set_cathode_offset_switch(SwitchState::Disconnected);

        let voltage_result =
            calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    pub fn test_cathode_offset_current<'a, const DONTCARE: HeaterState>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
        // spi_bus: &'a mut PayloadSPIController,
        // debug_writer: &mut SerialWriter<USCI>,
        // serial_reader: &mut Rx<USCI>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 99_440;
        let mut current_accuracy: Fxd = Fxd::ZERO;

        payload.set_cathode_offset_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (0..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100 - output_percentage)
                * (CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS)
                + output_percentage * (CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS))
                / 100;

            payload.set_cathode_offset_voltage(output_voltage_mv);


            let expected_voltage_mv: u32 = output_voltage_mv; // assume zero error between target voltage and actual voltage
            let expected_current_ua: i16 = ((1000 * expected_voltage_mv)
                / (hvdc_mock::MOCK_CATHODE_OFFSET_RESISTANCE_OHMS + CATHODE_SENSE_RESISTANCE_OHMS))
                as i16;
            println!("Expected current is: {}mA", expected_current_ua);

            //Manually measure the current
            print!("Measure current and input (in uA): ");
            let actual_current_ua = read_num(&mut payload.serial_reader);
            println!("");

            // Measure current
            let measured_current_ua: i32 = payload.get_cathode_offset_current_microamps();
            println!("Measured current is: {}uA", measured_current_ua);

            //Determine accuracy
            let current_rpd = calculate_rpd(measured_current_ua, actual_current_ua);
            println!(
                "Calculated current millirpd: {}",
                (current_rpd * 1000).to_num::<i32>()
            );
            current_accuracy = in_place_average(current_accuracy, current_rpd, i as u16);
        }

        // Set back to zero
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS);
        payload.set_cathode_offset_switch(SwitchState::Disconnected);

        let current_result =
            calculate_performance_result("Cathode offset current", current_accuracy, 5, 20);
        current_result
    }

    pub fn test_tether_bias_voltage<'a, const DONTCARE: HeaterState>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        payload.set_tether_bias_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (10..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100 - output_percentage)
                * (TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS)
                + output_percentage * (TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS))
                / 100;
            println!("Target output voltage: {}mV", output_voltage_mv);

            // Set tether bias
            payload.set_tether_bias_voltage(output_voltage_mv);

            delay_cycles(10000); //settling time

            // Read tether bias voltage, current
            print!("Measure voltage and input (in mV): ");
            let measured_voltage_mv = read_num(&mut payload.serial_reader);
            println!("");

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            println!(
                "Tether mv: {}",
                payload.get_tether_bias_voltage_millivolts()
            );
            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
            println!("");
        }

        // Set back to zero
        payload.set_tether_bias_voltage(TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS);
        payload.set_tether_bias_switch(SwitchState::Disconnected);

        let voltage_result =
            calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    pub fn test_tether_bias_current<'a, const DONTCARE: HeaterState>(
        payload: &'a mut Payload<{ PayloadOn }, DONTCARE>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 98_000;
        let mut current_accuracy: Fxd = Fxd::ZERO;

        payload.set_tether_bias_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (10..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100 - output_percentage)
                * (TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS)
                + output_percentage * (TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS))
                / 100;

            payload.set_tether_bias_voltage(output_voltage_mv);

            let expected_voltage_mv: u32 = output_voltage_mv; // assume zero error between target voltage and actual voltage
            let expected_current_ua: i16 = ((1000 * expected_voltage_mv)
                / (hvdc_mock::MOCK_TETHER_BIAS_RESISTANCE_OHMS + TETHER_SENSE_RESISTANCE_OHMS))
                as i16;
            println!("Expected current is: {}uA", expected_current_ua);
            
            //Manually measure the current
            print!("Measure current and input (in uA): ");
            let actual_current_ua = read_num(&mut payload.serial_reader);
            println!("");
            
            // Measure current
            let measured_current_ua: i32 = payload.get_tether_bias_current_microamps();
            println!("Measured current is: {}uA", measured_current_ua);

            //Determine accuracy
            let current_rpd = calculate_rpd(measured_current_ua, actual_current_ua);
            println!(
                "Calculated current millirpd: {}",
                (current_rpd * 1000).to_num::<i32>()
            );
            current_accuracy = in_place_average(current_accuracy, current_rpd, i as u16);
        }

        // Set back to zero
        payload.set_tether_bias_voltage(TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS);
        payload.set_tether_bias_switch(SwitchState::Disconnected);

        let current_result =
            calculate_performance_result("Tether bias current", current_accuracy, 5, 20);
        current_result
    }

    pub fn test_heater_voltage(
        payload: &mut Payload<{ PayloadOn }, { HeaterOn }>,
    ) -> PerformanceResult<'_> {
        const NUM_MEASUREMENTS: usize = 10;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (0..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = (((100 - output_percentage)
                * (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32)
                + output_percentage * (HEATER_MAX_VOLTAGE_MILLIVOLTS as u32))
                / 100) as u16;

            // Set cathode voltage
            payload.set_heater_voltage(output_voltage_mv);

            println!("Target set to: {}mV", output_voltage_mv);

            print!("Measure voltage and input (in mV): ");
            let actual_voltage_mv = read_num(&mut payload.serial_reader);
            println!("");

            let measured_voltage_mv = payload.get_heater_voltage_millivolts();
            println!("Measured as: {}", measured_voltage_mv);

            let voltage_rpd = calculate_rpd(measured_voltage_mv as i32, actual_voltage_mv);
            println!(
                "Calculated voltage millirpd: {}",
                (voltage_rpd * 1000).to_num::<i32>()
            );
            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
            println!("");
        }

        let voltage_result =
            calculate_performance_result("Heater voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    pub fn test_heater_current<'a>(
        payload: &'a mut Payload<{ PayloadOn }, { HeaterOn }>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;

        let mut current_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (0..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = (((100 - output_percentage)
                * (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32)
                + output_percentage * (HEATER_MAX_VOLTAGE_MILLIVOLTS as u32))
                / 100) as u16;

            // Set heater voltage
            payload.set_heater_voltage(output_voltage_mv);
            println!("Set voltage to: {}mV", output_voltage_mv);
            delay_cycles(100_000); //settling time

            // Calculate expected voltage and current (only for reference)
            let expected_voltage_mv: u32 = output_voltage_mv as u32; // assume zero error between target voltage and actual voltage
            let expected_current_ma: i16 = ((1000 * expected_voltage_mv)
                / (heater_mock::CIRCUIT_AND_PROBE_RESISTANCE_MOHMS as u32))
                .min(heater_mock::POWER_LIMITED_MAX_CURRENT_MA.to_num())
                as i16;
            println!("Expected current is: {}mA", expected_current_ma);

            // Measure current
            let measured_current_ma: i16 = payload.get_heater_current_milliamps();
            println!("Measured current is: {}mA", measured_current_ma);

            //Manually measure the current
            println!("Measure current and input (in mA): ");
            let actual_current_ma = read_num(&mut payload.serial_reader);
            println!("");

            //Determine accuracy
            let current_rpd = calculate_rpd(measured_current_ma as i32, actual_current_ma);
            println!(
                "Calculated current millirpd: {}",
                (current_rpd * 1000).to_num::<i32>()
            );
            current_accuracy = in_place_average(current_accuracy, current_rpd, i as u16);
        }

        let current_result =
            calculate_performance_result("Heater current", current_accuracy, 5, 20);
        current_result
    }
    /// Setup: Place 1.2 ohm (10W+) resistor between pinpuller pins.
    ///
    /// Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry
    pub fn test_pinpuller_current<
        'a,
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
    >(
        payload: &'a mut Payload<DONTCARE1, DONTCARE2>,
    ) -> PerformanceResult<'a> {
        let mut current_accuracy: Fxd = Fxd::ZERO;
        let mut expected_current_ma: i16;
        let mut measured_current_ma: i16;
        let voltage_values_mv: [i32; 9] = [400, 800, 1200, 1600, 2000, 2400, 2800, 3200, 3300];
        let rp_sense: i32 = 82;
        let r122: i32 = 400;
        let probe_resistance: i32 = 10; // Measure resistance with multimeter
        let wirewound_res: i32 = 1200; // Measure resistance with multimeter
        let mosfets: i32 = 27 * 2;
        let wire_resistance: i32 = 100 + 130;
        let total_resistance = rp_sense + r122 + wirewound_res + mosfets + wire_resistance; // Units: mOhms

        // Select burn wire 1 to form current loop.
        payload.pinpuller_pins.burn_wire_2.set_high().ok();

        // Loop over 10 voltages (in mV: 400, 800, 1200, 1600, 2000, 2400, 2800, 3200, 3300)
        for (i, set_voltage) in voltage_values_mv.iter().enumerate() {
            // Asking user to set required voltage
            println!(
                "Set voltage on power supply to {} mV. Once set, press any key to continue",
                set_voltage
            );
            wait_for_any_packet(&mut payload.serial_reader);

            // Obtain expected (I = V/R) and measured current in mA
            expected_current_ma = ((set_voltage * 1000) / total_resistance) as i16;
            measured_current_ma = payload.get_pinpuller_current_milliamps() as i16;
            // User inputs actual current from manual measurement
            println!("Measure current and input (in mA): ");
            let actual_current_ma = read_num(&mut payload.serial_reader) as i16;

            // Print results
            println!("Expected current is {} mA", expected_current_ma);
            println!("Measured current is {} mA", measured_current_ma);
            println!("Actual current is {} mA", actual_current_ma);

            // Calculate RPD and accuracy
            let current_rpd = calculate_rpd(measured_current_ma as i32, actual_current_ma as i32);
            println!(
                "Calculated current millirpd: {}",
                (current_rpd * 1000).to_num::<i32>()
            );
            current_accuracy = in_place_average(current_accuracy, current_rpd, i as u16);
        }

        PerformanceResult::default()
    }

    pub fn thermal_chamber_temp_sensors_test<
        'a,
        const DONTCARE1: PayloadState,
        const DONTCARE2: HeaterState,
        USCI: SerialUsci,
    >(
        payload: &mut Payload<{ DONTCARE1 }, { DONTCARE2 }>,
        spi_bus: &mut PayloadSPIController,
        debug_writer: &'a mut SerialWriter<USCI>,
        serial_reader: &'a mut Rx<USCI>,
    ) -> ! {
        // Does not return

        const TEMP_SENSORS: [(TemperatureSensor, &str); 8] = [
            (LMS_EMITTER_TEMPERATURE_SENSOR, "LMS Emitter"),
            (LMS_RECEIVER_TEMPERATURE_SENSOR, "LMS Receiver"),
            (MSP430_TEMPERATURE_SENSOR, "MSP430"),
            (HEATER_SUPPLY_TEMPERATURE_SENSOR, "Heater supply"),
            (HVDC_SUPPLIES_TEMPERATURE_SENSOR, "HVDC Supplies"),
            (TETHER_MONITORING_TEMPERATURE_SENSOR, "Tether monitoring"),
            (TETHER_CONNECTOR_TEMPERATURE_SENSOR, "Tether connector"),
            (MSP_3V3_TEMPERATURE_SENSOR, "MSP 3V3 supply"),
        ];

        // Prompt to setup thermal chamber
        uwriteln!(debug_writer, "Thermal Chamber Test").ok();
        uwriteln!(debug_writer, "--------------------").ok();
        uwriteln!(
            debug_writer,
            "Press any key to begin reading temperatures and then begin thermal chamber cycling"
        )
        .ok();
        wait_for_any_packet(serial_reader);

        // Loop to continuously read temperature values
        // 8 temperature sensor values will be printed every second or so
        // INFINITE loop so manually turn off power supply to exit loop.
        loop {
            for (n, (sensor, name)) in TEMP_SENSORS.iter().enumerate() {
                let tempr = payload.get_temperature_kelvin(sensor) as i16;
                uwrite!(debug_writer, "{}: ", name).ok();
                uwriteln!(
                    debug_writer,
                    "{}",
                    tempr - (CELCIUS_TO_KELVIN_OFFSET as i16)
                )
                .ok();
            }
            uwriteln!(debug_writer, "").ok();
            delay_cycles(1_000_000);
        }
    }
}

/// Functional test result.
pub struct SensorResult<'a> {
    name: &'a str,
    result: bool,
}
// Define how to print a SensorResult
impl ufmt::uDisplay for SensorResult<'_> {
    fn fmt<W: uWrite + ?Sized>(&self, f: &mut ufmt::Formatter<W>) -> Result<(), W::Error> {
        uwrite!(f, "[").ok();
        match self.result {
            true => crate::serial::uwrite_coloured!(f, " OK ", Green),
            false => crate::serial::uwrite_coloured!(f, "FAIL", Red),
        };

        uwrite!(f, "] {}", self.name).ok();
        Ok(())
    }
}

/// Accuracy test result. Includes a name, a broad performance category (e.g. good, ok, bad), and a numerical accuracy
#[derive(Copy, Clone, Default)]
pub struct PerformanceResult<'a> {
    name: &'a str,
    performance: Performance,
    accuracy: Fxd, // relative percent difference / 2
}
impl PerformanceResult<'_> {
    fn default<'a>() -> PerformanceResult<'a> {
        PerformanceResult {
            name: "",
            performance: Performance::NotWorking,
            accuracy: Fxd::ZERO,
        }
    }
}
// Define how to print a PerformanceResult
impl ufmt::uDisplay for PerformanceResult<'_> {
    fn fmt<W: uWrite + ?Sized>(&self, f: &mut ufmt::Formatter<W>) -> Result<(), W::Error> {
        uwrite!(f, "[").ok();
        match self.performance {
            Performance::Nominal => crate::serial::uwrite_coloured!(f, " OK ", Green),
            Performance::Inaccurate => crate::serial::uwrite_coloured!(f, "INAC", Yellow),
            Performance::NotWorking => crate::serial::uwrite_coloured!(f, "FAIL", Red),
        };

        uwrite!(
            f,
            "] {}, {}% error",
            self.name,
            (100 * self.accuracy).printable()
        )
        .ok();
        Ok(())
    }
}

#[derive(Copy, Clone, Default)]
pub enum Performance {
    Nominal,
    Inaccurate,
    #[default]
    NotWorking,
}
