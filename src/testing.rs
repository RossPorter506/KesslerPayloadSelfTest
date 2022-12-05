// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention
// These tests often rely on a sensor and an actuator together, so they test multiple components at once
// Functional (pass/fail) tests
struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests{
    // Dependencies: Isolated 5V supply, tether ADC, isolators
    pub fn tether_adc_functional_test() -> FunctionalResult {
        // Ask to read channel X.
        // Return success if SPI packet valid
        todo!();
    }
    // Dependencies: Isolated 5V supply, temperature ADC, isolators
    pub fn temperature_adc_functional_test() -> FunctionalResult {
        // Ask to read channel X.
        // Return success if SPI packet valid
        todo!();
    }
    // Dependencies: Isolated 5V supply, misc ADC, isolators
    pub fn misc_adc_functional_test() -> FunctionalResult {
        // Ask to read channel X.
        // Return success if SPI packet valid
        todo!();
    }
    // Dependencies: OBC SPI
    pub fn obc_spi_functional_test() -> FunctionalResult {
        // Set interrupt on cs line(?)
        // Read spi data
        // Compare against actual value
        // Return true if recorded packet matches
        todo!();
    }
    // Dependencies: pinpuller, pinpuller current sensor, misc ADC
    pub fn pinpuller_functional_test() -> (FunctionalResult, FunctionalResult, FunctionalResult, FunctionalResult) {
        // Short or place small resistor between pinpuller lines
        // Enable each of the four redundant lines.
        // Measure current
        // Return success if current above X mA
        todo!();
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