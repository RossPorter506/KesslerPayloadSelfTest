use embedded_hal::digital::v2::OutputPin;
use libm::pow;
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use replace_with::replace_with;

use crate::delay_cycles;
use crate::sensors::{PayloadController, PayloadOn, PayloadState};
use crate::{spi::*, adc::*, digipot::*, dac::*};
use crate::pcb_mapping_v5::{pin_name_types::*, sensor_locations::*, power_supply_limits::*, power_supply_locations::*, peripheral_vcc_values::*, *};

// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention
// These tests often rely on a sensor and an actuator together, so they test multiple components at once
// Functional (pass/fail) tests
pub struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests{
    // Internal function to reduce code duplication
    fn test_adc_functional<CsPin: ADCCSPin, SENSOR:ADCSensor>(  adc: &mut ADC<CsPin, SENSOR>, 
                                                                spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>,
                                                                wanted_channel: ADCChannel) -> bool {
        let adc_channel_msb = ((wanted_channel as u32) & 0b100) >> 2;
        let rest_of_adc_channel = (wanted_channel as u32) & 0b11;
        adc.cs_pin.set_low().ok();
        // ADC takes four cycles to track signal. Nothing to do for first two.
        let zeroes_1 = spi_bus.receive(2);

        // Send first bit of channel. Receive third and fourth zero.
        let zeroes_2 = spi_bus.send_and_receive(2, adc_channel_msb);

        // Send other two channel bits. Receive beginning of IN0 - could have any value.
        spi_bus.send(2, rest_of_adc_channel);

        //Wait out the rest of the IN0 reading being sent to us
        spi_bus.receive(11);

        // ADC is now tracking IN7. Receive zeroes while it tracks
        let zeroes_3 = spi_bus.receive(4);

        //Finally receive ADC value from the channel we care about
        spi_bus.receive(12);

        adc.cs_pin.set_high().ok();

        zeroes_1 == 0 && zeroes_2 == 0 && zeroes_3 == 0
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: Isolated 5V supply, tether ADC, isolators
    pub fn tether_adc_functional_test(payload: &mut PayloadController<PayloadOn>, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> bool {
        Self::test_adc_functional(&mut payload.tether_adc, spi_bus, ADCChannel::IN7)
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: temperature ADC
    pub fn temperature_adc_functional_test<DONTCARE: PayloadState>(payload: &mut PayloadController<DONTCARE>, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> bool {
        Self::test_adc_functional(&mut payload.temperature_adc, spi_bus, ADCChannel::IN7)
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: misc ADC
    pub fn misc_adc_functional_test<DONTCARE: PayloadState>(payload: &mut PayloadController<DONTCARE>, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> bool {
        Self::test_adc_functional(&mut payload.misc_adc, spi_bus, ADCChannel::IN7)
    }

    // Dependencies: OBC SPI
    pub fn obc_spi_functional_test() -> bool {
        // Set interrupt on cs line(?)
        // Read spi data
        // Compare against actual value
        // Return true if recorded packet matches
        todo!();
    }

    // Dependencies: pinpuller, pinpuller current sensor, misc ADC
    // Setup: Place 2 ohm (10W+) resistor (e.g. 30J2R0E) between pinpuller terminals
    pub fn pinpuller_functional_test<DONTCARE:PayloadState>(   pins: &mut PinpullerActivationPins, 
                                        payload: &mut PayloadController<DONTCARE>,
                                        spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> (bool, bool, bool, bool) {
        const ON_MILLIAMP_THRESHOLD: u16 = 1000; // TODO: Figure out threshhold
        
        // Enable each of the four redundant lines.
        // Measure current
        // Return success if current above X mA
        pins.burn_wire_1.set_high().ok();
        let result1 = payload.get_pinpuller_current_milliamps(spi_bus) > ON_MILLIAMP_THRESHOLD;
        pins.burn_wire_1.set_low().ok();
        delay_cycles(1000);

        pins.burn_wire_1_backup.set_high().ok();
        let result2 = payload.get_pinpuller_current_milliamps(spi_bus) > ON_MILLIAMP_THRESHOLD;
        pins.burn_wire_1_backup.set_low().ok();
        delay_cycles(1000);

        pins.burn_wire_2.set_high().ok();
        let result3 = payload.get_pinpuller_current_milliamps(spi_bus) > ON_MILLIAMP_THRESHOLD;
        pins.burn_wire_2.set_low().ok();
        delay_cycles(1000);

        pins.burn_wire_2_backup.set_high().ok();
        let result4 = payload.get_pinpuller_current_milliamps(spi_bus) > ON_MILLIAMP_THRESHOLD;
        pins.burn_wire_2_backup.set_low().ok();
        
        (result1, result2, result3, result4)
    }

    // Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn heater_functional_test(payload: &mut PayloadController<PayloadOn>, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> bool {
        // Set heater voltage to minimum
        // To do this we need to temporarily take ownership (can't move out of borrowed reference) of the bus to change it's typestate to talk to digipot.
        // Alternative is to own the SPI bus rather than take a &mut, then return it alongside the bool. Neither option is really that clean.
        replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
            let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge(); //configure bus for digipot
            payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS, &mut spi_bus_); // set voltage
            spi_bus_.into_idle_high().into_sample_falling_edge() //return bus
        });
        delay_cycles(100_000);
        // Read heater voltage. Should be near zero.
        let min_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
        
        // Set heater voltage to maximum.
        replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
            let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge(); //configure bus for digipot
            payload.set_heater_voltage(HEATER_MAX_VOLTAGE_MILLIVOLTS, &mut spi_bus_); // set voltage
            spi_bus_.into_idle_high().into_sample_falling_edge() //return bus
        });
        delay_cycles(100_000);
        
        // Read heater voltage. Should be near max (TODO: verify)
        let max_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
        min_voltage_mv < 50 && max_voltage_mv > 10_000
    }
    // Dependencies: LMS power switches, misc ADC, LMS LEDs, LMS receivers
    pub fn lms_functional_test<DONTCARE:PayloadState>(payload: &mut PayloadController<DONTCARE>, lms_control: &mut TetherLMSPins, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult, PerformanceResult) {
        let mut ambient_counts: [u16; 3] = [0; 3];
        let mut on_counts: [u16; 3] = [0; 3];

        // Enable phototransistors
        lms_control.lms_receiver_enable.set_high().ok();

        // Record max voltage/light value
        ambient_counts[0] = payload.misc_adc.read_count_from(&LMS_RECEIVER_1_SENSOR, spi_bus);
        ambient_counts[1] = payload.misc_adc.read_count_from(&LMS_RECEIVER_2_SENSOR, spi_bus);
        ambient_counts[2] = payload.misc_adc.read_count_from(&LMS_RECEIVER_3_SENSOR, spi_bus);
        let ambient_variance = variance(&ambient_counts);

        // Enable LEDs
        lms_control.lms_led_enable.set_high().ok();

        // Record max voltage/light value
        on_counts[0] = payload.misc_adc.read_count_from(&LMS_RECEIVER_1_SENSOR, spi_bus);
        on_counts[1] = payload.misc_adc.read_count_from(&LMS_RECEIVER_2_SENSOR, spi_bus);
        on_counts[2] = payload.misc_adc.read_count_from(&LMS_RECEIVER_3_SENSOR, spi_bus);
        let on_variance = variance(&on_counts);

        lms_control.lms_receiver_enable.set_low().ok();
        lms_control.lms_led_enable.set_low().ok();

        // Do something with ambient_variance and on_variance
        todo!();
        
    }

}

fn variance<T: Copy + Into<f32> + Ord>(arr: &[T]) -> f32{
    let length = arr.iter().len();
    let average: f32 = arr.iter().map(|n| (*n).into()).sum();
    arr.iter().fold(0.0, |sum, n| sum + ((*n).into()-average)*((*n).into()-average) ) / (length as f32)
}

// DO NOT USE OUTSIDE OF 'replace_with'! WILL panic if called!
// Make sure your replace_with call is panic-free!!
#[allow(unreachable_code)]
fn default_payload_spi_bus() -> PayloadSPIBitBang<IdleHigh, SampleFallingEdge>{
    unreachable!(); // This will panic.
    let periph = msp430fr2355::Peripherals::take().unwrap(); //so will this 
    let pmm = Pmm::new(periph.PMM);
    let port4 = Batch::new(periph.P4).split(&pmm);
    PayloadSPIBitBangConfig::new(   port4.pin7.pulldown(),
                                    port4.pin6.to_output(),
                                    port4.pin5.to_output(),)
                                    .sck_idle_high()
                                    .sample_on_falling_edge()
                                    .create()
}

fn calculate_accuracy<T: Copy + Into<f64>>(measured:T, actual:T) -> f64 {
    libm::fabs(measured.into() - actual.into() / actual.into())
}
fn average<T: Copy + Into<f64>>(arr: &[T]) -> f64 {
    let mut cumulative_avg: f64 = 0.0;
    for (i, num) in arr.iter().enumerate() {
        cumulative_avg += ((*num).into() - cumulative_avg) / ((i+1) as f64);
    }
    cumulative_avg
}
fn calculate_performance_result(arr: &[f64], success_threshhold: f64, inaccurate_threshhold: f64) -> PerformanceResult {
    let acc = average(arr);

    match acc {
    x if x > success_threshhold    => PerformanceResult::Success(acc),
    x if x > inaccurate_threshhold => PerformanceResult::Inaccurate(acc),
                                      _ => PerformanceResult::NotWorking(acc),}
}

// Accuracy-based tests
pub struct AutomatedPerformanceTests {}
impl AutomatedPerformanceTests{
    // Dependencies: Isolated 5V supply, tether ADC, DAC, cathode offset supply, signal processing circuitry, isolators
    // Setup: Place a 100k resistor between exterior and cathode-
    pub fn test_cathode_offset(payload: &mut PayloadController<PayloadOn>, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        payload.pins.cathode_switch.set_high().ok(); // connect to exterior
        for (i, output_percentage) in (0..=100u8).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage_mv = ((CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS - CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS) as f32 * output_fraction) as u32;

            // Set cathode voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge();
                payload.set_cathode_offset_voltage(output_voltage_mv, &mut spi_bus_);
                spi_bus_.into_idle_high().into_sample_falling_edge()
            });
            delay_cycles(100_000); //settling time
            
            // Read cathode voltage 
            let cathode_offset_voltage_mv = payload.get_cathode_offset_voltage_millivolts(spi_bus);

            // Read cathode current (setup TBD)
            let cathode_offset_current_ua = payload.get_cathode_offset_current_microamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = output_voltage_mv as i32;
            let expected_current_ma: i32 = (output_voltage_mv / (TEST_RESISTANCE + TETHER_SENSE_RESISTANCE_OHMS)) as i32;

            voltage_accuracy_measurements[i] = calculate_accuracy(cathode_offset_voltage_mv, expected_voltage_mv);
            current_accuracy_measurements[i] = calculate_accuracy(cathode_offset_current_ua, expected_current_ma);
        }
        payload.pins.cathode_switch.set_low().ok();

        let voltage_result = calculate_performance_result(&voltage_accuracy_measurements, 0.95, 0.8);
        let current_result = calculate_performance_result(&current_accuracy_measurements, 0.95, 0.8);
        (voltage_result, current_result)
    }
    // Almost identical code, feels bad man
    // Dependencies: isolated 5V supply, tether ADC, DAC, tether bias supply, signal processing circuitry, isolators
    // Setup: Place a 100k resistor between tether and cathode-
    pub fn test_tether_bias(payload: &mut PayloadController<PayloadOn>, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        payload.pins.tether_switch.set_high().ok(); // connect to tether
        for (i, output_percentage) in (0..=100u8).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage_mv = ((TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS - TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS) as f32 * output_fraction) as u32;

            // Set cathode voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge();
                payload.set_tether_bias_voltage(output_voltage_mv, &mut spi_bus_);
                spi_bus_.into_idle_high().into_sample_falling_edge()
            });
            delay_cycles(100_000); //settling time
            
            // Read cathode voltage 
            let tether_bias_voltage_mv = payload.get_tether_bias_voltage_millivolts(spi_bus);

            // Read cathode current (setup TBD)
            let tether_bias_current_ua = payload.get_tether_bias_current_microamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = output_voltage_mv as i32;
            let expected_current_ma: i32 = (output_voltage_mv / (TEST_RESISTANCE + TETHER_SENSE_RESISTANCE_OHMS)) as i32;

            voltage_accuracy_measurements[i] = calculate_accuracy(tether_bias_voltage_mv, expected_voltage_mv);
            current_accuracy_measurements[i] = calculate_accuracy(tether_bias_current_ua, expected_current_ma as i32);
        }
        payload.pins.tether_switch.set_low().ok();

        let voltage_result = calculate_performance_result(&voltage_accuracy_measurements, 0.95, 0.8);
        let current_result = calculate_performance_result(&current_accuracy_measurements, 0.95, 0.8);
        (voltage_result, current_result)
    }
    // Generic version, couldn't get working due to overlapping borrows for function pointers
    /*fn test_generic_voltage_current(supply_max: u32, supply_min: u32, success_threshhold: f64, inaccurate_threshhold: f64,
                                    read_voltage_fn: &dyn Fn(&mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> i32,
                                    read_current_fn: &dyn Fn(&mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> i32,
                                    set_voltage_fn:  &dyn Fn(u32, &mut PayloadSPIBitBang<IdleLow, SampleRisingEdge>),
                                    calculate_current_fn: &dyn Fn(i32) -> i32,
                                    spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        let mut voltage_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        for (i, output_percentage) in (0..=100u8).step_by(NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage = ((supply_max - supply_min) as f32 * output_fraction) as u32;

            // Set voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge();
                set_voltage_fn(output_voltage, &mut spi_bus_);
                spi_bus_.into_idle_high().into_sample_falling_edge()
            });
            delay_cycles(100_000); //settling time
            
            // Read cathode voltage 
            let tether_bias_voltage_mv = read_voltage_fn(spi_bus);

            // Read cathode current (setup TBD)
            let tether_bias_current_ua = read_current_fn(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage: i32 = output_voltage as i32;
            let expected_current: i32 = calculate_current_fn(output_voltage as i32);

            voltage_accuracy_measurements[i] = calculate_accuracy(tether_bias_voltage_mv, expected_voltage);
            current_accuracy_measurements[i] = calculate_accuracy(tether_bias_current_ua, expected_current as i32);
        }

        let voltage_accuracy = average(&voltage_accuracy_measurements);
        let current_accuracy = average(&current_accuracy_measurements);

        let voltage_result = match voltage_accuracy {
            x if x > success_threshhold     => PerformanceResult::Success(voltage_accuracy),
            x if x > inaccurate_threshhold => PerformanceResult::Inaccurate(voltage_accuracy),
            _                                   => PerformanceResult::NotWorking(voltage_accuracy),
        };
        let current_result = match current_accuracy {
            x if x > success_threshhold    => PerformanceResult::Success(current_accuracy),
            x if x > inaccurate_threshhold => PerformanceResult::Inaccurate(current_accuracy),
            _                                   => PerformanceResult::NotWorking(current_accuracy),
        };

        (voltage_result, current_result)
    }*/
    // Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    // Test configuration: 10 ohm resistor across heater+ and heater-
    pub fn test_heater(payload: &mut PayloadController<PayloadOn>, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        const HEATER_RESISTANCE: f32 = 10.0 + 0.01;
        const HEATER_MAX_POWER: f32 = 1.0;
        const MAXIMUM_ON_CURRENT: f32 = HEATER_MAX_VOLTAGE_MILLIVOLTS as f32 / HEATER_RESISTANCE; 
        let heater_power_limited_max_current: f64 = 1000.0 * libm::sqrt((HEATER_MAX_POWER / HEATER_RESISTANCE) as f64);
        let mut voltage_accuracy_measurements: [f64; NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64; NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        for (i, output_percentage) in (0..=100u8).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage = ((HEATER_MAX_VOLTAGE_MILLIVOLTS - HEATER_MIN_VOLTAGE_MILLIVOLTS) as f32 * output_fraction) as u16;

            // Set cathode voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge();
                payload.set_heater_voltage(output_voltage, &mut spi_bus_);
                spi_bus_.into_idle_high().into_sample_falling_edge()
            });
            delay_cycles(100_000); //settling time
            
            // Read voltage 
            let heater_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);

            // Read current (setup TBD)
            let heater_current_ma = payload.get_heater_current_milliamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage: u16 = output_voltage;
            let expected_current: i16 = (libm::fmin((MAXIMUM_ON_CURRENT * output_fraction) as f64, heater_power_limited_max_current)) as i16;

            voltage_accuracy_measurements[i] = calculate_accuracy(heater_voltage_mv, expected_voltage);
            current_accuracy_measurements[i] = calculate_accuracy(heater_current_ma, expected_current);
        }

        let voltage_result = calculate_performance_result(&voltage_accuracy_measurements, 0.95, 0.8);
        let current_result = calculate_performance_result(&current_accuracy_measurements, 0.95, 0.8);
        (voltage_result, current_result)
    }
    
    // Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry
    // Setup: Place 2 ohm (10W+) resistor between pinpuller pins. // TODO
    pub fn test_pinpuller_current_sensor<DONTCARE:PayloadState>(payload: &mut PayloadController<DONTCARE>, p_pins: &mut PinpullerActivationPins, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> PerformanceResult {
        const EXPECTED_OFF_CURRENT: u16 = 0;
        const MOSFET_R_ON_RESISTANCE: f32 = 0.03; // Verify(?)
        const PINPULLER_MOCK_RESISTANCE: f32 = 2.0;
        const SENSE_RESISTANCE: f32 = 0.4;
        const EXPECTED_ON_CURRENT: u16 = (PINPULLER_VOLTAGE_MILLIVOLTS as f32 / (PINPULLER_MOCK_RESISTANCE + SENSE_RESISTANCE + MOSFET_R_ON_RESISTANCE*2.0)) as u16;
        
        let mut accuracy_measurements: [f64; 5] = [0.0; 5];

        accuracy_measurements[0] = calculate_accuracy(payload.get_pinpuller_current_milliamps(spi_bus), 0);

        // Activate pinpuller
        // Measure current
        p_pins.burn_wire_1.set_high().ok();
        accuracy_measurements[1] = calculate_accuracy(payload.get_pinpuller_current_milliamps(spi_bus), EXPECTED_ON_CURRENT);
        p_pins.burn_wire_1.set_low().ok();
        delay_cycles(1000);

        p_pins.burn_wire_1_backup.set_high().ok();
        accuracy_measurements[2] = calculate_accuracy(payload.get_pinpuller_current_milliamps(spi_bus), EXPECTED_ON_CURRENT);
        p_pins.burn_wire_1_backup.set_low().ok();
        delay_cycles(1000);

        p_pins.burn_wire_2.set_high().ok();
        accuracy_measurements[3] = calculate_accuracy(payload.get_pinpuller_current_milliamps(spi_bus), EXPECTED_ON_CURRENT);
        p_pins.burn_wire_2.set_low().ok();
        delay_cycles(1000);

        p_pins.burn_wire_2_backup.set_high().ok();
        accuracy_measurements[4] = calculate_accuracy(payload.get_pinpuller_current_milliamps(spi_bus), EXPECTED_ON_CURRENT);
        p_pins.burn_wire_2_backup.set_low().ok();

        calculate_performance_result(&accuracy_measurements, 0.95, 0.8)
    }    
}

// Tests that require human intervention during the test
// Functional (pass/fail) tests
pub struct ManualFunctionalTests{}
impl ManualFunctionalTests{
    // Dependencies: endmass switches
    pub fn endmass_switches_functional_test() -> FunctionalResult {
        // Read switches
        // Ask user to depress both switches
        // Read switches again
        // Return true if different
        todo!();
    }
    // Dependencies: pinpuller
    pub fn pinpuller_functional_test() -> (FunctionalResult, FunctionalResult, FunctionalResult, FunctionalResult) {
        // Enable each of the four redundant lines.
        // Manually check resistance across pinpuller pins
        todo!();
    }
    // Dependencies: LMS power switches
    pub fn lms_power_switch_functional_test() -> (FunctionalResult, FunctionalResult) {
        // Enable LMS LED EN
        // Measure resistance between J1 pin 2A/B and GND
        // Enable LMS Receiver EN
        // Manually measure resistance between J1 pin 3A/B and GND
        // Query user for resistance 
        // Return true if resistance less than 1-10 ohms
        todo!();
    }
    // Dependencies: pinpuller sense
    pub fn pinpuller_sense_functional_test() -> FunctionalResult {
        // Read pinpuller sense lines
        // Short pinpuller sense lines
        // Read pinpuller sense lines again
        // Return true if different
        todo!();
    }
}

// Accuracy-based tests
pub struct ManualPerformanceTests{}
impl ManualPerformanceTests{
    // Dependencies: Isolated 5V supply, Tether ADC, signal processing circuitry, isolators
    pub fn test_repeller() -> PerformanceResult {
        // Manually set repeller voltage (setup TBD)
        // Read repeller voltage
        // Calculate expected voltage
        // Return success if error within 10%
        todo!();
    }
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
    }
    // Dependencies: Isolated 5V supply, DAC, isolators
    pub fn test_dac() -> PerformanceResult {
        // Set channel X.
        // Manually measure voltage
        // Query user for voltage
        // Return success if voltage within 10%
        todo!();
    }
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
    }
}

pub enum PerformanceResult{
    Success(f64), // accuracy error in %
    Inaccurate(f64),
    NotWorking(f64),
}

// Nice names for bool values
pub enum FunctionalResult{
    Functional=1,
    NonFunctional=0,
}