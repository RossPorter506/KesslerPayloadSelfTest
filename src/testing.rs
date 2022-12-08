use embedded_hal::digital::v2::OutputPin;

use crate::{spi::{PayloadSPI, SckIdleHigh}, adc::{TetherADC, ADCChannel, ADC, ADCSensor, MiscADC}, pcb_mapping_v5::{PinpullerPins, AdcCsPin, PINPULLER_CURRENT_SENSOR}};

// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention
// These tests often rely on a sensor and an actuator together, so they test multiple components at once
// Functional (pass/fail) tests
struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests{
    fn test_adc_functional<CsPin: AdcCsPin, SENSOR:ADCSensor>(  adc: &mut ADC<CsPin, SENSOR>, 
                                                                spi_bus: &mut impl PayloadSPI<SckIdleHigh>,
                                                                wanted_channel: ADCChannel) -> bool {
        let adc_channel_msb = ((wanted_channel as u32) & 0b100) >> 2;
        let rest_of_adc_channel = (wanted_channel as u32) & 0b11;
        let _ = adc.cs_pin.set_low();
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

        let _ = adc.cs_pin.set_high();

        zeroes_1 == 0 && zeroes_2 == 0 && zeroes_3 == 0
    }
    // Dependencies: Isolated 5V supply, tether ADC, isolators
    // Ask to read channel 7.
    // Return success if SPI packet valid
    pub fn tether_adc_functional_test(tether_adc: &mut TetherADC, spi_bus: &mut impl PayloadSPI<SckIdleHigh>) -> bool {
        Self::test_adc_functional(tether_adc, spi_bus, ADCChannel::IN7)
    }
    // Dependencies: temperature ADC
    pub fn temperature_adc_functional_test(temperature_adc: &mut TetherADC, spi_bus: &mut impl PayloadSPI<SckIdleHigh>) -> bool {
        // Ask to read channel 7.
        // Return success if SPI packet valid
        Self::test_adc_functional(temperature_adc, spi_bus, ADCChannel::IN7)
    }
    // Dependencies: misc ADC
    pub fn misc_adc_functional_test(misc_adc: &mut TetherADC, spi_bus: &mut impl PayloadSPI<SckIdleHigh>) -> bool {
        // Ask to read channel 7.
        // Return success if SPI packet valid
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
                                        spi_bus: &mut impl PayloadSPI<SckIdleHigh>) -> (bool, bool, bool, bool) {
        // Short or place small resistor between pinpuller lines
        // Enable each of the four redundant lines.
        // Measure current
        // Return success if current above X mA
        let _ = pins.burn_wire_1.set_high();
        let result1 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        let _ = pins.burn_wire_1.set_low();

        let _ = pins.burn_wire_1_backup.set_high();
        let result2 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        let _ = pins.burn_wire_1_backup.set_low();

        let _ = pins.burn_wire_2.set_high();
        let result3 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        let _ = pins.burn_wire_2.set_low();

        let _ = pins.burn_wire_2_backup.set_high();
        let result4 = adc.read_count_from(&PINPULLER_CURRENT_SENSOR, spi_bus) > 1000; // TODO: Figure out threshhold
        let _ = pins.burn_wire_2_backup.set_low();
        
        (result1, result2, result3, result4)
    }
}

// Accuracy-based tests
struct AutomatedPerformanceTests {}
impl AutomatedPerformanceTests{
    // Dependencies: Isolated 5V supply, tether ADC, DAC, cathode offset supply, signal processing circuitry, isolators
    pub fn test_cathode_offset() -> (PerformanceResult, PerformanceResult) {
        // Set cathode voltage
        // Read cathode voltage 
        // Read cathode current (setup TBD)
        // Calculate expected voltage and current
        // Return success if closed loop error within 10%
        todo!();
    }
    // Dependencies: isolated 5V supply, tether ADC, DAC, tether bias supply, signal processing circuitry, isolators
    pub fn test_tether_bias() -> (PerformanceResult, PerformanceResult) {
        // Set cathode voltage
        // Read cathode voltage 
        // Read cathode current (setup TBD)
        // Calculate expected voltage and current
        // Return success if closed loop error within 10%
        todo!();
    }
    // Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn test_heater() -> (PerformanceResult, PerformanceResult) {
        // Place 10(?) ohm resistor (1W+) across heater pins
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
struct ManualFunctionalTests{}
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
struct ManualPerformanceTests{}
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
    Success(f32), // accuracy error in %
    Inaccurate(f32),
    NotWorking(f32),
}

// Nice names for bool values
pub enum FunctionalResult{
    Functional=1,
    NonFunctional=0,
}