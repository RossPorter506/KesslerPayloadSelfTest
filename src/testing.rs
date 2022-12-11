use embedded_hal::digital::v2::OutputPin;
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use replace_with::replace_with;

use crate::delay_cycles;
use crate::sensors::PayloadController;
use crate::{spi::*, adc::*, digipot::*, dac::*};
use crate::pcb_mapping_v5::{sensor_equations::*, sensor_locations::*, power_supply_limits::*, power_supply_locations::*, power_supply_equations::*, *};
// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention
// These tests often rely on a sensor and an actuator together, so they test multiple components at once
// Functional (pass/fail) tests
pub struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests{
    // Internal function to reduce code duplication
    fn test_adc_functional<CsPin: AdcCsPin, SENSOR:ADCSensor>(  adc: &mut ADC<CsPin, SENSOR>, 
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
    pub fn tether_adc_functional_test(tether_adc: &mut TetherADC, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> bool {
        Self::test_adc_functional(tether_adc, spi_bus, ADCChannel::IN7)
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: temperature ADC
    pub fn temperature_adc_functional_test(temperature_adc: &mut TetherADC, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> bool {
        Self::test_adc_functional(temperature_adc, spi_bus, ADCChannel::IN7)
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: misc ADC
    pub fn misc_adc_functional_test(misc_adc: &mut TetherADC, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> bool {
        Self::test_adc_functional(misc_adc, spi_bus, ADCChannel::IN7)
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
    pub fn pinpuller_functional_test(   pins: &mut PinpullerPins, 
                                        adc: &mut MiscADC, 
                                        spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> (bool, bool, bool, bool) {
        // Short or place small resistor between pinpuller lines
        // Enable each of the four redundant lines.
        // Measure current
        // Return success if current above X mA
        pins.burn_wire_1.set_high().ok();
        let result1 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        pins.burn_wire_1.set_low().ok();

        pins.burn_wire_1_backup.set_high().ok();
        let result2 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        pins.burn_wire_1_backup.set_low().ok();

        pins.burn_wire_2.set_high().ok();
        let result3 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        pins.burn_wire_2.set_low().ok();

        pins.burn_wire_2_backup.set_high().ok();
        let result4 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        pins.burn_wire_2_backup.set_low().ok();
        
        (result1, result2, result3, result4)
    }

    // Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn heater_functional_test(tether_adc: &mut TetherADC, digipot: &mut Digipot, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> bool {
        // Read heater voltage. Should be near zero.
        let zero_count = tether_adc.read_count_from(&HEATER_VOLTAGE_SENSOR, spi_bus);
        // Set heater voltage to maximum.

        // Temporarily take ownership of the bus to change it's typestate to talk to digipot.
        // Alternative is to own the SPI bus rather than take a &mut, then return it alongside the bool. Neither option is really that clean.
        replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
            let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge(); //configure bus for digipot
            digipot.set_channel_to_count(HEATER_DIGIPOT_CHANNEL, 0x00, &mut spi_bus_); // read digipot
            spi_bus_.into_idle_high().into_sample_falling_edge() //return bus
        });
        
        // Read heater voltage. Should be near max (TODO: verify)
        let max_count = tether_adc.read_count_from(&HEATER_VOLTAGE_SENSOR, spi_bus);
        zero_count < 100 && max_count > 4000
    }

}

// DO NOT USE OUTSIDE OF replace_with! WILL panic if called!
// Make sure your replace_with call is panic-free!!
#[allow(unreachable_code)]
fn default_payload_spi_bus() -> PayloadSPIBitBang<IdleHigh, SampleFallingEdge>{
    unreachable!(); // This will panic.
    let periph = msp430fr2355::Peripherals::take().unwrap(); 
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
fn average<T: Copy + Into<f64>>(arr:&[T]) -> f64 {
    let mut cumulative_avg: f64 = 0.0;
    for (i, num) in arr.iter().enumerate() {
        cumulative_avg += ((*num).into() - cumulative_avg) / ((i+1) as f64);
    }
    cumulative_avg
}

// Accuracy-based tests
pub struct AutomatedPerformanceTests {}
impl AutomatedPerformanceTests{
    // Dependencies: Isolated 5V supply, tether ADC, DAC, cathode offset supply, signal processing circuitry, isolators
    pub fn test_cathode_offset(payload: &mut PayloadController, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        let mut voltage_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        for (i, output_percentage) in (0..=100u8).step_by(NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage = ((CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS - CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS) as f32 * output_fraction) as u32;

            // Set cathode voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge();
                payload.set_cathode_offset_voltage(output_voltage, &mut spi_bus_);
                spi_bus_.into_idle_high().into_sample_falling_edge()
            });
            delay_cycles(100_000); //settling time
            
            // Read cathode voltage 
            let cathode_offset_voltage_mv = payload.get_cathode_offset_voltage_millivolts(spi_bus);

            // Read cathode current (setup TBD)
            let cathode_offset_current_ua = payload.get_cathode_offset_current_microamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage:i32 = output_voltage as i32;
            let expected_current:i32 = todo!();

            voltage_accuracy_measurements[i] = calculate_accuracy(cathode_offset_voltage_mv, expected_voltage);
            current_accuracy_measurements[i] = calculate_accuracy(cathode_offset_current_ua, expected_current as i32);
        }

        let voltage_accuracy = average(&voltage_accuracy_measurements);
        let current_accuracy = average(&current_accuracy_measurements);

        let voltage_result = match voltage_accuracy {
            x if x > 0.95 => PerformanceResult::Success(voltage_accuracy),
            x if x > 0.80 => PerformanceResult::Inaccurate(voltage_accuracy),
            _                  => PerformanceResult::NotWorking(voltage_accuracy),
        };
        let current_result = match current_accuracy {
            x if x > 0.95 => PerformanceResult::Success(current_accuracy),
            x if x > 0.80 => PerformanceResult::Inaccurate(current_accuracy),
            _                  => PerformanceResult::NotWorking(current_accuracy),
        };

        (voltage_result, current_result)
    }
    // Almost identical code, feels bad man
    // Dependencies: isolated 5V supply, tether ADC, DAC, tether bias supply, signal processing circuitry, isolators
    pub fn test_tether_bias(payload: &mut PayloadController, spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFallingEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        let mut voltage_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        for (i, output_percentage) in (0..=100u8).step_by(NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage = ((TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS - TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS) as f32 * output_fraction) as u32;

            // Set cathode voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low().into_sample_rising_edge();
                payload.set_tether_bias_voltage(output_voltage, &mut spi_bus_);
                spi_bus_.into_idle_high().into_sample_falling_edge()
            });
            delay_cycles(100_000); //settling time
            
            // Read cathode voltage 
            let tether_bias_voltage_mv = payload.get_tether_bias_voltage_millivolts(spi_bus);

            // Read cathode current (setup TBD)
            let tether_bias_current_ua = payload.get_tether_bias_current_microamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage:i32 = output_voltage as i32;
            let expected_current:i32 = todo!();

            voltage_accuracy_measurements[i] = calculate_accuracy(tether_bias_voltage_mv, expected_voltage);
            current_accuracy_measurements[i] = calculate_accuracy(tether_bias_current_ua, expected_current as i32);
        }

        let voltage_accuracy = average(&voltage_accuracy_measurements);
        let current_accuracy = average(&current_accuracy_measurements);

        let voltage_result = match voltage_accuracy {
            x if x > 0.95 => PerformanceResult::Success(voltage_accuracy),
            x if x > 0.80 => PerformanceResult::Inaccurate(voltage_accuracy),
            _                  => PerformanceResult::NotWorking(voltage_accuracy),
        };
        let current_result = match current_accuracy {
            x if x > 0.95 => PerformanceResult::Success(current_accuracy),
            x if x > 0.80 => PerformanceResult::Inaccurate(current_accuracy),
            _                  => PerformanceResult::NotWorking(current_accuracy),
        };

        (voltage_result, current_result)
    }
    // Generic version, couldn't get working due to overlapping borrows of 'payload'
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

            // Set cathode voltage
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
    pub fn test_heater() -> (PerformanceResult, PerformanceResult) {
        // Set heater voltage
        // Read heater voltage, current
        // Return success if error within 10%
        todo!();
    }
    // Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry (does this one actually have any circuitry?)
    pub fn test_pinpuller_current_sensor() -> PerformanceResult {
        // Short pinpuller pins
        // Activate pinpuller
        // Measure current
        // Return success if measured current within 10%
        todo!();
    }
    // Dependencies: LMS power switches, misc ADC, LMS LEDs, LMS receivers
    pub fn test_lms() -> (PerformanceResult, PerformanceResult, PerformanceResult) {
        // Attach LMS board
        // Enable power rails.
        // Record max voltage/light value
        // Enable LEDs
        // Record max voltage/light value
        // Return success if enabled value double or more of default
        todo!();
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