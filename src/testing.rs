use embedded_hal::digital::v2::{OutputPin, InputPin};
use msp430fr2x5x_hal::serial::{SerialUsci, Rx};
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use ufmt::{uWrite, uwrite, uwriteln};

use crate::delay_cycles;
use crate::payload::{PayloadController, PayloadState, PayloadState::*, HeaterState, HeaterState::*, SwitchState};
use crate::serial::{SerialWriter, wait_for_any_packet};
#[allow(unused_imports)]
use crate::{spi::{*, SckPolarity::*, SckPhase::SampleFirstEdge}, adc::*, digipot::*, dac::*};
#[allow(unused_imports)]
use crate::pcb_mapping::{pin_name_types::*, sensor_locations::*, power_supply_limits::*, power_supply_locations::*, peripheral_vcc_values::*, *};
use crate::serial::read_num;
use fixed::{self, FixedI64};

//Macros to only print if debug_print feature is enabled
macro_rules! dbg_uwriteln {
    ($first:tt $(, $( $rest:tt )* )?) => {    
        #[cfg(feature = "debug_print")]
        {uwrite!($first, "[....] ").ok(); uwriteln!($first, $( $($rest)* )*).ok();}
    }
}
#[allow(unused_macros)]
macro_rules! dbg_uwrite {
    ($first:tt $(, $( $rest:tt )* )?) => {    
        #[cfg(feature = "debug_print")]
        {uwrite!($first, "[....] ").ok(); uwrite!($first, $( $($rest)* )*).ok();}
    }
}

// We use this type a lot. 
/// 64 bits long, 32 fractional bits, signed. 
/// 
/// Range: -2,147,483,648 to 2,147,483,647. 
/// 
/// Delta: 2.3283064e-10 = 0.00000000023283064
type Fxd = FixedI64::<32>;

/// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention.
/// These tests often rely on a sensor and an actuator together, so they test multiple components at once.
/// Functional tests are pass/fail.
pub struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests{
    pub fn full_system_test<USCI:SerialUsci>(
            payload: &mut PayloadController<{PayloadOn}, {HeaterOn}>, 
            pinpuller_pins: &mut PinpullerActivationPins, 
            lms_pins: &mut TetherLMSPins,
            spi_bus: &mut PayloadSPIController, 
            serial: &mut SerialWriter<USCI>){

        uwriteln!(serial, "==== Automated Functional Tests Start ====").ok();
        for adc_test_fn in [Self::tether_adc_functional_test, Self::temperature_adc_functional_test, Self::misc_adc_functional_test].iter(){
            uwriteln!(serial, "{}", adc_test_fn(payload, spi_bus.borrow())).ok();
        }

        for pinpuller_lane in Self::pinpuller_functional_test(pinpuller_pins, payload, spi_bus).iter() {
            uwriteln!(serial, "{}", pinpuller_lane).ok();
        }

        uwriteln!(serial, "{}", Self::heater_functional_test(payload, spi_bus, serial)).ok();

        for lms_channel in Self::lms_functional_test(payload, lms_pins, spi_bus).iter(){
            uwriteln!(serial, "{}", lms_channel).ok();
        }

        uwriteln!(serial, "==== Automated Functional Tests Complete ====").ok();
    }
    // Internal function to reduce code duplication
    fn test_adc_functional<CsPin: ADCCSPin, SENSOR: ADCSensor, const VCC: u16>(  
            adc: &mut ADC<CsPin, SENSOR, VCC>, 
            spi_bus: &mut impl PayloadSPI<{IdleHigh}, {SampleFirstEdge}>,
            wanted_channel: ADCChannel) -> bool {
        let payload = (wanted_channel as u32) << (NUM_CYCLES_FOR_TWO_READINGS - NUM_ADDRESS_BITS - NUM_LEADING_ZEROES); // see adc.rs read_count_from
        let result = spi_bus.send_receive(NUM_CYCLES_FOR_TWO_READINGS, payload, &mut adc.cs_pin);
        let zeroes = result & 0xF000_F000;

        zeroes == 0
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    /// 
    /// Dependencies: Isolated 5V supply, tether ADC, isolators
    pub fn tether_adc_functional_test<'a, const DONTCARE: HeaterState>(
            payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
            spi_bus: &'a mut impl PayloadSPI<{IdleHigh}, {SampleFirstEdge}>) -> SensorResult<'a> {
        let result = Self::test_adc_functional(&mut payload.tether_adc, spi_bus, ADCChannel::IN7);
        SensorResult { name: "Tether ADC", result }
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    /// 
    /// Dependencies: temperature ADC
    pub fn temperature_adc_functional_test<'a, const DONTCARE1: PayloadState, const DONTCARE2: HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            spi_bus: &'a mut impl PayloadSPI<{IdleHigh}, {SampleFirstEdge}>) -> SensorResult<'a> {
        let result = Self::test_adc_functional(&mut payload.temperature_adc, spi_bus, ADCChannel::IN7);
        SensorResult { name: "Temperature ADC", result }
    }

    /// Ask to read channel 7.
    /// Return success if SPI packet valid
    /// 
    /// Dependencies: misc ADC
    pub fn misc_adc_functional_test<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            spi_bus: &'a mut impl PayloadSPI<{IdleHigh}, {SampleFirstEdge}>) -> SensorResult<'a> {
        let result =Self::test_adc_functional(&mut payload.misc_adc, spi_bus, ADCChannel::IN7);
        SensorResult { name: "Misc ADC", result }
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
    pub fn pinpuller_functional_test<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState>(   
            pins: &'a mut PinpullerActivationPins, 
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>,
            spi_bus: &'a mut PayloadSPIController) -> [SensorResult<'a>; 4] {
        const ON_MILLIAMP_THRESHOLD: u16 = 1000; // TODO: Figure out threshhold
        let mut results = [false; 4];
        
        // Enable each of the four redundant lines.
        // Measure current
        // Return success if current above X mA
        let mut pin_arr: [&mut dyn OutputPin<Error=void::Void>; 4] = [  
            &mut pins.burn_wire_1, 
            &mut pins.burn_wire_1_backup, 
            &mut pins.burn_wire_2, 
            &mut pins.burn_wire_2_backup];
        
        for (n, pin) in pin_arr.iter_mut().enumerate(){
            pin.set_high().ok();
            results[n] = payload.get_pinpuller_current_milliamps(spi_bus) > ON_MILLIAMP_THRESHOLD;
            pin.set_low().ok();
            delay_cycles(1000);
        }
        
        [SensorResult{name: "Pinpuller channel 1",        result: results[0]}, 
         SensorResult{name: "Pinpuller channel 1 backup", result: results[1]}, 
         SensorResult{name: "Pinpuller channel 2",        result: results[2]}, 
         SensorResult{name: "Pinpuller channel 2 backup", result: results[3]}]
    }

    /// Set the heater to the minimum, wait 0.1s and read voltage. Set to maximum, wait 0.1s, read voltage. 
    /// Check these values are within 10% of expected values.
    /// 
    /// Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn heater_functional_test<'a, USCI: SerialUsci>(
            payload: &mut PayloadController<{PayloadOn}, {HeaterOn}>, 
            spi_bus: &mut PayloadSPIController,
            debug_writer: &mut SerialWriter<USCI>) -> SensorResult<'a> {

        // Set heater to min
        payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS, spi_bus); // set voltage
        delay_cycles(100_000);
        // Read voltage
        let min_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
        dbg_uwriteln!(debug_writer, "Min voltage set to {}. Read as {}, expected at most {}", HEATER_MIN_VOLTAGE_MILLIVOLTS, min_voltage_mv, (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32) * 11/10);
        
        // Set heater to max
        payload.set_heater_voltage(HEATER_MAX_VOLTAGE_MILLIVOLTS, spi_bus); // set voltage
        delay_cycles(100_000);
        // Read voltage
        let max_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
        dbg_uwriteln!(debug_writer, "Max voltage set to {}. Read as {}, expected at least {}", HEATER_MAX_VOLTAGE_MILLIVOLTS, max_voltage_mv, (HEATER_MAX_VOLTAGE_MILLIVOLTS as u32) * 9/10);

        // Set heater back to min and give time to settle
        payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS, spi_bus); // set voltage
        delay_cycles(1_000_000);

        SensorResult{name: "Heater", result: ((min_voltage_mv as u32) < (HEATER_MIN_VOLTAGE_MILLIVOLTS as u32) * 11/10) 
                                          && ((max_voltage_mv as u32) > (HEATER_MAX_VOLTAGE_MILLIVOLTS as u32) * 9/10) }
    }
    
    /// Enable receivers, record ambient values. Enable LEDs, record values. Return ok if on_value > 2 * ambient.
    /// 
    /// Setup: Connect LMS board, test in a room with minimal (or at least uniform) IR interference. 
    /// Dependencies: LMS power switches, misc ADC, LMS LEDs, LMS receivers
    pub fn lms_functional_test<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            lms_control: &'a mut TetherLMSPins, 
            spi_bus: &'a mut PayloadSPIController) -> [SensorResult<'a>;3] {
        let mut ambient_counts: [u16; 3] = [0; 3];
        let mut on_counts: [u16; 3] = [0; 3];

        // Enable phototransistors
        lms_control.lms_receiver_enable.set_high().ok();

        // Record max voltage/light value
        for (n, sensor) in [LMS_RECEIVER_1_SENSOR, LMS_RECEIVER_2_SENSOR, LMS_RECEIVER_2_SENSOR].iter().enumerate() {
            ambient_counts[n] = payload.misc_adc.read_count_from(sensor, spi_bus.borrow());
        }

        // Enable LEDs
        lms_control.lms_led_enable.set_high().ok();

        // Record max voltage/light value
        for (n, sensor) in [LMS_RECEIVER_1_SENSOR, LMS_RECEIVER_2_SENSOR, LMS_RECEIVER_2_SENSOR].iter().enumerate() {
            on_counts[n] = payload.misc_adc.read_count_from(sensor, spi_bus.borrow());
        }

        lms_control.lms_receiver_enable.set_low().ok();
        lms_control.lms_led_enable.set_low().ok();

        [SensorResult{name: "Length measurement system 1", result: (on_counts[0] > 2*ambient_counts[0])}, 
         SensorResult{name: "Length measurement system 2", result: (on_counts[1] > 2*ambient_counts[1])}, 
         SensorResult{name: "Length measurement system 3", result: (on_counts[2] > 2*ambient_counts[2])}]
    }

}

/// Rather than using percent error (which isn't defined when the actual value is zero), we use Relative Percent Difference (RPD).
/// Outputs are between -1 and 1. Values near zero are close to percentage error, but 1 means measured is infinitely larger than actual, -1 means measured is infinitely smaller than actual.
fn calculate_rpd(measured: i32, actual: i32) -> Fxd {
    if actual == 0 && measured == 0{
        return Fxd::ZERO;
    }
    let actual = Fxd::from(actual);
    let measured = Fxd::from(measured);

    // (measured - actual) / measured.abs() + actual.abs()
    (measured - actual).checked_div(measured.abs() + actual.abs())
        .unwrap_or_else(|| if measured - actual > 0 {Fxd::MAX} else {Fxd::MIN}) // This only fires on overflow. 
    // Ideally we could use saturating_div instead of checked_div and unwrap_or_else
    // but the panic_never condition fails even if we check denominator is non-zero first...
}
/// Iteratively updates an average with a new value
fn in_place_average(acc: Fxd, new: Fxd, n: u16) -> Fxd{
    //acc + ((new - acc) / Fxd::from(n+1))
    acc + ((new - acc).checked_div(Fxd::from(n+1))
        .unwrap_or(Fxd::ZERO)) // unwrap_or should never fire, since n+1 > 0 when n is unsigned.
} 

fn calculate_performance_result(name: &str, rpd: Fxd, succ_percent: u8, inacc_percent: u8) -> PerformanceResult<'_> {
    let performance = match rpd.abs() {
        x if x < Fxd::from(succ_percent) / 100  => Performance::Nominal,
        x if x < Fxd::from(inacc_percent) / 100 => Performance::Inaccurate,
        _ => Performance::NotWorking};
    PerformanceResult{name, performance, accuracy: rpd}
}

/// Calculates a square root approximation using Newton's method. Panics on negative values.
const fn fixed_sqrt(x: Fxd) -> Fxd {
    if x.is_negative() {panic!();} 
    else if x.is_zero() {return Fxd::ZERO;}
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
impl AutomatedPerformanceTests{
    pub fn full_system_test<USCI:SerialUsci>(
            payload: &mut PayloadController<{PayloadOn}, {HeaterOn}>, 
            pinpuller_pins: &mut PinpullerActivationPins,
            spi_bus: &mut PayloadSPIController, 
            serial: &mut SerialWriter<USCI>){
        uwriteln!(serial, "==== Automatic Performance Tests Start ====").ok();
        // Each of these three fn's takes the same arguments and both return a voltage and current result
        let fn_arr = [Self::test_cathode_offset, Self::test_tether_bias, Self::test_heater];
        for sensor_fn in fn_arr.iter(){
            for sensor_result in sensor_fn(payload, spi_bus, serial).iter(){
                uwriteln!(serial, "{}", sensor_result).ok();
            }
        }
        uwriteln!(serial, "{}", Self::test_pinpuller_current_sensor(payload, pinpuller_pins, spi_bus)).ok();

        uwriteln!(serial, "==== Automatic Performance Tests Complete ====\n").ok();
    }
    /// Setup: Place a 100k resistor between exterior and cathode-
    /// 
    /// Dependencies: Isolated 5V supply, tether ADC, DAC, cathode offset supply, signal processing circuitry, isolators
    pub fn test_cathode_offset<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
            payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
            spi_bus: &'a mut PayloadSPIController,
            debug_writer: &mut SerialWriter<USCI>) -> [PerformanceResult<'a>; 2] {
        
        let [voltage_accuracy, current_accuracy] = Self::test_hvdc_supply(
            &PayloadController::set_cathode_offset_switch, 
            &PayloadController::get_cathode_offset_voltage_millivolts, 
            &PayloadController::get_cathode_offset_current_microamps, 
            &PayloadController::set_cathode_offset_voltage, 
            CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS, 
            CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS, 
            payload,
            spi_bus, 
            debug_writer);

        let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        let current_result = calculate_performance_result("Cathode offset current", current_accuracy, 5, 20);
        [voltage_result, current_result]
    }

    /// Setup: Place a 100k resistor between tether and cathode-
    /// 
    /// Dependencies: isolated 5V supply, tether ADC, DAC, tether bias supply, signal processing circuitry, isolators
    pub fn test_tether_bias<'a, const DONTCARE: HeaterState, USCI: SerialUsci>(
            payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
            spi_bus: &'a mut PayloadSPIController,
            debug_writer: &mut SerialWriter<USCI>) -> [PerformanceResult<'a>; 2] {
        
        let [voltage_accuracy, current_accuracy] = Self::test_hvdc_supply(
            &PayloadController::set_tether_bias_switch, 
            &PayloadController::get_tether_bias_voltage_millivolts, 
            &PayloadController::get_tether_bias_current_microamps, 
            &PayloadController::set_tether_bias_voltage, 
            TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS, 
            TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS, 
            payload,
            spi_bus, 
            debug_writer);

        let voltage_result = calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
        let current_result = calculate_performance_result("Tether bias current", current_accuracy, 5, 20);
        [voltage_result, current_result]
    }
    
    /// Internal function to reduce code duplication.
    fn test_hvdc_supply<const DONTCARE: HeaterState, USCI:SerialUsci>(
        set_switch_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, SwitchState), 
        measure_voltage_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, &mut PayloadSPIController) -> i32,
        measure_current_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, &mut PayloadSPIController) -> i32,
        set_voltage_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, u32, &mut PayloadSPIController),
        supply_min: u32,
        supply_max: u32,
        payload: &mut PayloadController<{PayloadOn}, DONTCARE>,
        spi_bus: &mut PayloadSPIController,
        debug_writer: &mut SerialWriter<USCI>) -> [Fxd; 2] {
        
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        const SENSE_RESISTANCE: u32 = 1; // Both supplies use the same sense resistor value
        const TEST_START_PERCENT: u32 = 10;
        const TEST_END_PERCENT: u32 = 100;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;
        let mut current_accuracy: Fxd = Fxd::ZERO;

        set_switch_fn(payload, SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (TEST_START_PERCENT..=TEST_END_PERCENT).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let set_voltage_mv: u32 = ((100-output_percentage)*(supply_min) + output_percentage*(supply_max)) / 100;
            dbg_uwriteln!(debug_writer, "Target output voltage: {}mV", set_voltage_mv);

            // Set cathode voltage
            set_voltage_fn(payload, set_voltage_mv, spi_bus);

            delay_cycles(100_000); //settling time
            
            // Read voltage, current
            let measured_voltage_mv = measure_voltage_fn(payload, spi_bus);
            let measured_current_ua = measure_current_fn(payload, spi_bus);
            dbg_uwriteln!(debug_writer, "Measured output voltage: {}mV", measured_voltage_mv);
            dbg_uwriteln!(debug_writer, "Measured output current: {}uA", measured_current_ua);

            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = set_voltage_mv as i32;
            let expected_current_ua: i32 = ((1000 * set_voltage_mv) / (TEST_RESISTANCE + SENSE_RESISTANCE)) as i32;

            dbg_uwriteln!(debug_writer, "Expected output voltage: {}mV", expected_voltage_mv);
            dbg_uwriteln!(debug_writer, "Expected output current: {}uA", expected_current_ua);

            let voltage_rpd = calculate_rpd(measured_voltage_mv, expected_voltage_mv);
            let current_rpd = calculate_rpd(measured_current_ua, expected_current_ua);

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd,i as u16);
            current_accuracy = in_place_average(current_accuracy, current_rpd,i as u16);
            dbg_uwriteln!(debug_writer, "");
        }

        // Set back to zero
        set_voltage_fn(payload, supply_min, spi_bus);

        set_switch_fn(payload, SwitchState::Disconnected);

        [voltage_accuracy, current_accuracy]
    }
    
    /// Setup: 10 ohm resistor across heater+ and heater-
    /// 
    /// Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn test_heater<'a, USCI: SerialUsci>(
            payload: &'a mut PayloadController<{PayloadOn}, {HeaterOn}>, 
            spi_bus: &'a mut PayloadSPIController, 
            debug_writer: &mut SerialWriter<USCI> ) -> [PerformanceResult<'a>; 2] {
        const NUM_MEASUREMENTS: usize = 10;
        
        let mut voltage_accuracy: Fxd = Fxd::ZERO;
        let mut current_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (0..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = (((100-output_percentage)*(HEATER_MIN_VOLTAGE_MILLIVOLTS as u32) + output_percentage*(HEATER_MAX_VOLTAGE_MILLIVOLTS as u32)) / 100) as u16;

            // Set cathode voltage
            payload.set_heater_voltage(output_voltage_mv, spi_bus);

            dbg_uwriteln!(debug_writer, "Set voltage to: {}mV", output_voltage_mv);
            delay_cycles(100_000); //settling time
            
            // Read voltage, current
            let heater_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
            dbg_uwriteln!(debug_writer, "Read voltage as: {}mV", heater_voltage_mv);
            let heater_current_ma = payload.get_heater_current_milliamps(spi_bus);
            dbg_uwriteln!(debug_writer, "Read current as: {}mA", heater_current_ma);

            // Calculate expected voltage and current
            let expected_voltage_mv: u16 = output_voltage_mv;
            let expected_current_ma: i16 = (expected_voltage_mv as u32 * 1000 / heater_mock::CIRCUIT_RESISTANCE_MOHMS as u32)
                .min(heater_mock::POWER_LIMITED_MAX_CURRENT_MA.to_num()) as i16;
            dbg_uwriteln!(debug_writer, "Expected current is: {}mA", expected_current_ma);

            let voltage_rpd = calculate_rpd(heater_voltage_mv as i32, expected_voltage_mv as i32);
            dbg_uwriteln!(debug_writer, "Voltage milliRPD is: {}", (voltage_rpd*1000).to_num::<i32>());
            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd, i as u16);
            current_accuracy = in_place_average(current_accuracy, calculate_rpd(heater_current_ma as i32, expected_current_ma as i32), i as u16);
            dbg_uwriteln!(debug_writer, "");
        }

        let voltage_result = calculate_performance_result("Heater voltage", voltage_accuracy, 5, 20);
        let current_result = calculate_performance_result("Heater current", current_accuracy, 5, 20);
        [voltage_result, current_result]
    }
    
    /// Setup: Place 1.2 ohm (10W+) resistor between pinpuller pins. // TODO
    ///
    /// Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry
    pub fn test_pinpuller_current_sensor<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            p_pins: &'a mut PinpullerActivationPins, 
            spi_bus: &'a mut PayloadSPIController) -> PerformanceResult<'a> {
        
        let mut accuracy: Fxd = Fxd::ZERO;

        // For each pin, activate the pinpuller through that channel and measure the current
        let mut pin_list: [&mut dyn OutputPin<Error = void::Void>; 4] = [
            &mut p_pins.burn_wire_1, 
            &mut p_pins.burn_wire_1_backup, 
            &mut p_pins.burn_wire_2, 
            &mut p_pins.burn_wire_2_backup];
        
        for (n, pin) in pin_list.iter_mut().enumerate() {
            pin.set_high().ok();
            let measured_current = payload.get_pinpuller_current_milliamps(spi_bus);
            accuracy = in_place_average(accuracy, 
                calculate_rpd(measured_current as i32, pinpuller_mock::EXPECTED_ON_CURRENT.to_num()), 
                n as u16);
            pin.set_low().ok();
            delay_cycles(1000);
        }
        
        calculate_performance_result("Pinpuller current sense",  accuracy,  5, 20)
    }    
}

/// Tests that require human intervention. These are pass/fail tests.
pub struct ManualFunctionalTests{}
impl ManualFunctionalTests{
    pub fn full_system_test<USCI: SerialUsci>(
            pins: &DeploySensePins,
            serial_writer: &mut SerialWriter<USCI>, 
            serial_reader: &mut Rx<USCI>){

        uwriteln!(serial_writer, "==== Manual Functional Tests Start ====").ok();

        for result in Self::endmass_switches_functional_test(pins, serial_writer, serial_reader).iter(){
            uwriteln!(serial_writer, "{}", result).ok();
        }
        
        uwriteln!(serial_writer, "==== Manual Functional Tests Complete ====").ok();
    }
    /// Dependencies: endmass switches
    pub fn endmass_switches_functional_test<'a, 'b, USCI: SerialUsci>(
            pins: &DeploySensePins,
            serial_writer: &'a mut SerialWriter<USCI>, 
            serial_reader: &'a mut Rx<USCI>) -> [SensorResult<'b>; 2] {

        uwriteln!(serial_writer, "Depress switches").ok();
        wait_for_any_packet(serial_reader);

        // Note: is_low/is_high is infallible, so ignore the unwraps
        let is_depressed_arr: [bool; 2] = [pins.endmass_sense_1.is_low().unwrap_or(false), pins.endmass_sense_2.is_low().unwrap_or(false)];

        uwriteln!(serial_writer, "Release switches").ok();
        
        wait_for_any_packet(serial_reader);

        let is_released_arr: [bool; 2] = [pins.endmass_sense_1.is_high().unwrap_or(false), pins.endmass_sense_2.is_high().unwrap_or(false)];

        [SensorResult {name: "Endmass switch 1", result: (is_depressed_arr[0] && is_released_arr[0])},
         SensorResult {name: "Endmass switch 2", result: (is_depressed_arr[1] && is_released_arr[1])}]
    }

    // Dependencies: pinpuller
    pub fn pinpuller_functional_test <'a, USCI: SerialUsci> (pins: &mut PinpullerActivationPins, serial_writer: &mut SerialWriter<USCI>, serial_reader: &mut Rx<USCI>) -> [PerformanceResult<'a>; 4] {
        // Enable each of the four redundant lines.
        
        let mut pin_arr: [(&mut dyn OutputPin<Error=void::Void>, &str); 4] = [
            (&mut pins.burn_wire_1,          "Burn Wire 1"),
            (&mut pins.burn_wire_2,          "Burn Wire 2"),
            (&mut pins.burn_wire_1_backup,   "Burn Wire 1 backup"),
            (&mut pins.burn_wire_2_backup,   "Burn Wire 2 backup"),
        ];
        let mut result: [PerformanceResult; 4] = Default::default();
        // Manually check resistance(?) across pinpuller pins
        for (n, (pin, name)) in pin_arr.iter_mut().enumerate(){
            pin.set_high().ok();
            let measured = read_num(serial_writer, serial_reader);
            uwriteln!(serial_writer, "{} active.", name).ok();
            uwriteln!(serial_writer, "Please enter current:").ok();
            pin.set_low().ok();
            delay_cycles(1000);
            result[n] = calculate_performance_result(name, calculate_rpd(measured, pinpuller_mock::EXPECTED_ON_CURRENT.to_num()), 5, 20);
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
mod pinpuller_mock {
    use super::{Fxd, PINPULLER_VOLTAGE_MILLIVOLTS};

    const MOSFET_R_ON_RESISTANCE: Fxd = Fxd::lit("0.03"); // Verify(?)
    const PINPULLER_MOCK_RESISTANCE: Fxd = Fxd::lit("1.2");
    const SENSE_RESISTANCE: Fxd = Fxd::lit("0.4");
    const CIRCUIT_RESISTANCE: Fxd = 
        PINPULLER_MOCK_RESISTANCE
        .unwrapped_add(SENSE_RESISTANCE)
        .unwrapped_add(MOSFET_R_ON_RESISTANCE)
        .unwrapped_add(MOSFET_R_ON_RESISTANCE);
    const NUM_PINS: usize = 4;
    pub const EXPECTED_ON_CURRENT: Fxd = 
        Fxd::const_from_int(PINPULLER_VOLTAGE_MILLIVOLTS as i64) 
        .unwrapped_div(CIRCUIT_RESISTANCE);
}

/// Values associated with mock heater tests
mod heater_mock {
    use super::{Fxd, fixed_sqrt};

    const MOCK_HEATER_RESISTANCE_MOHMS: u16 = 10_000;
    const PROBE_RESISTANCE_MOHMS: u16 = 90;
    pub const CIRCUIT_RESISTANCE_MOHMS: u16 = MOCK_HEATER_RESISTANCE_MOHMS + super::HEATER_SENSE_RESISTANCE_MILLIOHMS as u16; // heater resistance + shunt resistor
    pub const CIRCUIT_AND_PROBE_RESISTANCE_MOHMS: u16 = CIRCUIT_RESISTANCE_MOHMS + PROBE_RESISTANCE_MOHMS;
    const HEATER_MAX_POWER_MWATTS: u16 = 1000; // TODO: Verify?
    
    pub const POWER_LIMITED_MAX_CURRENT_MA: Fxd = fixed_sqrt( Fxd::const_from_int(HEATER_MAX_POWER_MWATTS as i64).unwrapped_div_int(CIRCUIT_RESISTANCE_MOHMS as i64))
        .unwrapped_mul_int(1000); //sqrt(heater_max_power_mw / circuit_resistance_mohm) * 1000;
}

fn test_temperature_sensors_against_known_temp<'a, const DONTCARE1:PayloadState, const DONTCARE2:HeaterState, USCI:SerialUsci>(
        room_temp_k: u16,
        payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>,
        serial_writer: &'a mut SerialWriter<USCI>,
        serial_reader: &'a mut Rx<USCI>, 
        spi_bus: &'a mut PayloadSPIController) -> [PerformanceResult<'static>; 8]{
    
    const TEMP_SENSORS: [(TemperatureSensor, &str); 8] = [
        (LMS_EMITTER_TEMPERATURE_SENSOR,        "LMS Emitter"),
        (LMS_RECEIVER_TEMPERATURE_SENSOR,       "LMS Receiver"),
        (MSP430_TEMPERATURE_SENSOR,             "MSP430"),
        (HEATER_SUPPLY_TEMPERATURE_SENSOR,      "Heater supply"),
        (HVDC_SUPPLIES_TEMPERATURE_SENSOR,      "HVDC Supplies"),
        (TETHER_MONITORING_TEMPERATURE_SENSOR,  "Tether monitoring"),
        (TETHER_CONNECTOR_TEMPERATURE_SENSOR,   "Tether connector"),
        (MSP_3V3_TEMPERATURE_SENSOR,            "MSP 3V3 supply"),
    ];
    
    let mut output_arr: [PerformanceResult; 8] = [PerformanceResult::default(); 8];
    for (n, (sensor, name)) in TEMP_SENSORS.iter().enumerate() {
        let tempr = payload.get_temperature_kelvin(sensor, spi_bus);
        let accuracy = calculate_rpd(tempr as i32, room_temp_k as i32);
        output_arr[n] = calculate_performance_result(name, 
                                                     accuracy, 
                                                     5, 
                                                     20)
    }

    output_arr
}

const CELCIUS_TO_KELVIN_OFFSET: u16 = 273;
// Accuracy-based tests
pub struct ManualPerformanceTests{}
impl ManualPerformanceTests{
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
    fn query_room_temp<USCI:SerialUsci>(serial_writer: &mut SerialWriter<USCI>, serial_reader: &mut Rx<USCI>) -> u16 {
        uwriteln!(serial_writer, "Enter current temp (in celcius)").ok();
        let celcius_num = read_num(serial_writer, serial_reader);
        (celcius_num + CELCIUS_TO_KELVIN_OFFSET as i32) as u16
    }
    pub fn two_point_test_temperature_sensor_test<'a, USCI: SerialUsci, const DONTCARE: HeaterState>( 
            payload: &'a mut PayloadController<{PayloadOff}, DONTCARE>, // Minimise heat generation
            serial_writer: &'a mut SerialWriter<USCI>,
            serial_reader: &'a mut Rx<USCI>, 
            spi_bus: &'a mut PayloadSPIController) -> [PerformanceResult<'a>; 8]{


        let mut room_temp_k: u16 = Self::query_room_temp(serial_writer, serial_reader);

        let arr1 = test_temperature_sensors_against_known_temp(room_temp_k, payload, serial_writer, serial_reader, spi_bus);

        room_temp_k = Self::query_room_temp(serial_writer, serial_reader);

        let arr2 = test_temperature_sensors_against_known_temp(room_temp_k, payload, serial_writer, serial_reader, spi_bus);

        let mut result_arr: [PerformanceResult; 8] = [PerformanceResult::default(); 8];

        for (n, (result1, result2)) in arr1.iter().zip(arr2.iter()).enumerate(){
            let accuracy = (result1.accuracy + result2.accuracy) / 2;
            result_arr[n] = calculate_performance_result(result1.name, 
                                                         accuracy, 
                                                         5, 
                                                         20)
        }
        result_arr
    }

    /// Dependencies: Isolated 5V supply, DAC, isolators
    pub fn test_dac<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
        payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
        spi_bus: &'a mut impl PayloadSPI<{IdleLow}, {SampleFirstEdge}>,
        debug_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI> ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 5;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (1..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = ((output_percentage * DAC_VCC_VOLTAGE_MILLIVOLTS as u32) / 100) as u16;
            let dac_count = DAC::voltage_to_count(output_voltage_mv);
            uwriteln!(debug_writer, "Target output voltage: {}mV. DAC count: {}", output_voltage_mv, dac_count).ok();

            // Set DAC voltage
            payload.dac.send_command(DACCommand::WriteToAndUpdateRegisterX, 
                DACChannel::ChannelC, 
                dac_count, 
                spi_bus);

            delay_cycles(1000); //settling time
            
            // Read cathode voltage, current
            uwrite!(debug_writer, "Measure voltage and enter in mV: ").ok();
            let measured_voltage_mv = read_num(debug_writer, serial_reader);
            uwriteln!(debug_writer, "").ok();

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            uwriteln!(debug_writer, "Calculated voltage millirpd: {}", (voltage_rpd*1000).to_num::<i32>()).ok();

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd,i as u16);
        }

        // Set back to zero
        payload.dac.send_command(DACCommand::WriteToAndUpdateRegisterX, 
            DACChannel::ChannelA, 
            DAC::voltage_to_count(0), 
            spi_bus);

        let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
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
    pub fn test_cathode_offset_voltage<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
        payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
        spi_bus: &'a mut PayloadSPIController,
        debug_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI> ) -> PerformanceResult<'a> {
        
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        payload.set_cathode_offset_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (10..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100-output_percentage)*(CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS) 
                                            + output_percentage *(CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS)) / 100;
            uwriteln!(debug_writer, "Target output voltage: {}mV", output_voltage_mv).ok();

            // Set cathode voltage
            payload.set_cathode_offset_voltage(output_voltage_mv, spi_bus);

            delay_cycles(10000); //settling time
            
            // Read cathode voltage, current
            uwrite!(debug_writer, "Measure voltage and input (in mV): ").ok();
            let measured_voltage_mv = read_num(debug_writer, serial_reader);
            uwriteln!(debug_writer, "").ok();

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            uwriteln!(debug_writer, "Calculated voltage millirpd: {}", (voltage_rpd*1000).to_num::<i32>()).ok();

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd,i as u16);
        }

        // Set back to zero
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS, spi_bus);
        payload.set_cathode_offset_switch(SwitchState::Disconnected);

        let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
        voltage_result
    }

    pub fn test_tether_bias_voltage<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
        payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
        spi_bus: &'a mut PayloadSPIController,
        debug_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI> ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        payload.set_tether_bias_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (10..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u32 = ((100-output_percentage)*(TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS) 
                                            + output_percentage *(TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS)) / 100;
            uwriteln!(debug_writer, "Target output voltage: {}mV", output_voltage_mv).ok();

            // Set tether bias
            payload.set_tether_bias_voltage(output_voltage_mv, spi_bus);

            delay_cycles(10000); //settling time
            
            // Read tether bias voltage, current
            uwrite!(debug_writer, "Measure voltage and input (in mV): ").ok();
            let measured_voltage_mv = read_num(debug_writer, serial_reader);
            uwriteln!(debug_writer, "").ok();

            let voltage_rpd = calculate_rpd(measured_voltage_mv, output_voltage_mv as i32);
            uwriteln!(debug_writer, "Calculated voltage millirpd: {}", (voltage_rpd*1000).to_num::<i32>()).ok();

            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd,i as u16);
        }

        // Set back to zero
        payload.set_tether_bias_voltage(TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS, spi_bus);
        payload.set_tether_bias_switch(SwitchState::Disconnected);

        let voltage_result = calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
        voltage_result
    }
    pub fn test_heater_voltage<'a, USCI: SerialUsci>(
        payload: &'a mut PayloadController<{PayloadOn}, {HeaterOn}>, 
        spi_bus: &'a mut PayloadSPIController, 
        debug_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI>) -> PerformanceResult<'a> {
        
        const NUM_MEASUREMENTS: usize = 10;
        let mut voltage_accuracy: Fxd = Fxd::ZERO;

        for (i, output_percentage) in (0..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = (((100-output_percentage)*(HEATER_MIN_VOLTAGE_MILLIVOLTS as u32) + output_percentage*(HEATER_MAX_VOLTAGE_MILLIVOLTS as u32)) / 100) as u16;

            // Set cathode voltage
            payload.set_heater_voltage(output_voltage_mv, spi_bus);

            uwriteln!(debug_writer, "Target set to: {}mV", output_voltage_mv).ok();

            uwrite!(debug_writer, "Measure voltage and input (in mV): ").ok();
            let actual_voltage_mv = read_num(debug_writer, serial_reader);
            uwriteln!(debug_writer, "").ok();

            let measured_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);

            let voltage_rpd = calculate_rpd(measured_voltage_mv as i32, actual_voltage_mv);
            uwriteln!(debug_writer, "Calculated voltage millirpd: {}", (voltage_rpd*1000).to_num::<i32>()).ok();
            voltage_accuracy = in_place_average(voltage_accuracy, voltage_rpd,i as u16);
        }

        let voltage_result = calculate_performance_result("Heater voltage", voltage_accuracy, 5, 20);
        voltage_result
    }
    
    pub fn test_heater_current<'a, USCI: SerialUsci>(
        payload: &'a mut PayloadController<{ PayloadOn }, { HeaterOn }>,
        spi_bus: &'a mut PayloadSPIController,
        debug_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI>,
    ) -> PerformanceResult<'a> {
        const NUM_MEASUREMENTS: usize = 10;
        
        let mut current_accuracy: Fxd = Fxd::ZERO;
    
        for (i, output_percentage) in (0..=100u32).step_by(100 / NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv: u16 = 
                (((100 - output_percentage)*(HEATER_MIN_VOLTAGE_MILLIVOLTS as u32)
                + output_percentage*(HEATER_MAX_VOLTAGE_MILLIVOLTS as u32)) / 100) as u16;
    
            // Set heater voltage
            payload.set_heater_voltage(output_voltage_mv, spi_bus);
            uwriteln!(debug_writer, "Set voltage to: {}mV", output_voltage_mv).ok();
            delay_cycles(100_000); //settling time
    
            // Calculate expected voltage and current (only for reference)
            let expected_voltage_mv: u32 = output_voltage_mv as u32; // assume zero error between target voltage and actual voltage
            let expected_current_ma: i16 = ((1000*expected_voltage_mv) / (heater_mock::CIRCUIT_AND_PROBE_RESISTANCE_MOHMS as u32))
                .min(heater_mock::POWER_LIMITED_MAX_CURRENT_MA.to_num()) as i16;
            dbg_uwriteln!(debug_writer, "Expected current is: {}mA", expected_current_ma);

            // Measure current
            let measured_current_ma: i16 = payload.get_heater_current_milliamps(spi_bus);
            dbg_uwriteln!(debug_writer, "Measured current is: {}mA", measured_current_ma);
    
            //Manually measure the current
            uwrite!(debug_writer,"Measure current and input (in mA): ").ok();
            let actual_current_ma = read_num(debug_writer, serial_reader);
            uwriteln!(debug_writer, "").ok();
    
            //Determine accuracy
            let current_rpd = calculate_rpd(measured_current_ma as i32, actual_current_ma);
            uwriteln!(debug_writer,"Calculated current millirpd: {}", (current_rpd * 1000).to_num::<i32>()).ok();
            current_accuracy = in_place_average(current_accuracy, current_rpd, i as u16);
        }
    
        let current_result = calculate_performance_result(
            "Heater current",
            current_accuracy,
            5,
            20,
        );
        current_result
    }
    /// Setup: Place 2 ohm (10W+) resistor between pinpuller pins.
    ///
    /// Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry
    pub fn test_pinpuller_current<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState, USCI: SerialUsci>(
        payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
        p_pins: &'a mut PinpullerActivationPins, 
        spi_bus: &'a mut PayloadSPIController,
        serial_writer: &mut SerialWriter<USCI>,
        serial_reader: &mut Rx<USCI>,) -> PerformanceResult<'a> {
    
        let mut accuracy: Fxd = Fxd::ZERO;

        // For each pin, activate the pinpuller through that channel and measure the current
        let mut pin_list: [&mut dyn OutputPin<Error = void::Void>; 4] = [
            &mut p_pins.burn_wire_1, 
            &mut p_pins.burn_wire_1_backup, 
            &mut p_pins.burn_wire_2, 
            &mut p_pins.burn_wire_2_backup];
        
        for (n, pin) in pin_list.iter_mut().enumerate() {
            pin.set_high().ok();

            uwrite!(serial_writer,"Measure current and input (in mA): ").ok();
            let actual_current_ma = read_num(serial_writer, serial_reader);
            uwriteln!(serial_writer, "").ok();

            let measured_current_ma = payload.get_pinpuller_current_milliamps(spi_bus);
            
            accuracy = in_place_average(accuracy, 
                calculate_rpd(measured_current_ma as i32, actual_current_ma), 
                n as u16);
            pin.set_low().ok();
            delay_cycles(1000);
        }

        calculate_performance_result("Pinpuller current sense",  accuracy,  5, 20)
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
        let result = match self.result {
            true => " OK ",
            false => "FAIL"};

        uwrite!(f, "[{}] {}", result, self.name).ok();
        Ok(())
    }
}

/// Accuracy test result. Includes a name, a broad performance category (e.g. good, ok, bad), and a numerical accuracy
#[derive(Copy, Clone, Default)]
pub struct PerformanceResult<'a>{
    name: &'a str, 
    performance: Performance,
    accuracy: Fxd, // relative percent difference / 2
}
impl PerformanceResult<'_>{
    fn default<'a>()-> PerformanceResult<'a> {
        PerformanceResult{name: "", performance: Performance::NotWorking, accuracy: Fxd::ZERO}
    }
}
// Define how to print a PerformanceResult
impl ufmt::uDisplay for PerformanceResult<'_> {
    fn fmt<W: uWrite + ?Sized>(&self, f: &mut ufmt::Formatter<W>) -> Result<(), W::Error> {
        let result = match self.performance {
            Performance::Nominal    => " OK ",
            Performance::Inaccurate => "INAC",
            Performance::NotWorking => "FAIL"};
        let percent_acc: i32 = (self.accuracy*100).to_num();
        let fractional_percent: i32 = (self.accuracy*10000).to_num::<i32>() - percent_acc*100;
        uwrite!(f, "[{}] {}, {}.{}% error", result, self.name, percent_acc, fractional_percent).ok();
        Ok(())
    }
}

#[derive(Copy, Clone, Default)]
pub enum Performance {
    Nominal,
    Inaccurate,
    #[default] NotWorking,
}