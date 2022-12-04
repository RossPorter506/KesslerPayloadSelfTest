//Test peripherals first

// Open-loop tests
pub fn test_tether_adc() -> Status{
    // Manually apply known voltage to channel.
    // Ask to read channel X.
    // Return success if SPI packet valid and accuracy within 10%
}
pub fn test_temperature_adc() -> Status{
    // Manually apply known voltage to channel.
    // Ask to read channel X.
    // Return success if SPI packet valid and accuracy within 10%
}
pub fn test_misc_adc() -> Status{
    // Manually apply known voltage to channel.
    // Ask to read channel X.
    // Return success if SPI packet valid and accuracy within 10%
}
pub fn test_dac() -> Status{
    // Set channel X.
    // Measure voltage by hand.
    // Query user about voltage
    // Return success if voltage within 10%
}
pub fn test_digipot() -> Status{
    // Set channel X
    // Measure resistance by hand.
    // Query user about resistance
    // Return success if resistance within 10%
}
pub fn test_endmass_switches() -> bool{
    // Ask user to depress both switches
    // Return true if sensed
}
pub fn test_pinpuller_sense() -> bool{
    // Short pinpuller sense lines
    // Return true if sensed
}
pub fn test_pinpuller() -> (bool, bool, bool, bool) {
    // Test each of the four redundant lines.
    // Manually check resistance across pinpuller pins
}
pub fn test_obc_spi() -> bool {
    // Set interrupt on cs line
    // Read spi data
    // Compare against actual value
    // Return true if recorded packet matches  
}

// Closed-loop tests
pub fn test_cathode_offset() -> (Status, Status) {
    // Set cathode voltage
    // Read cathode voltage 
    // Read cathode current (setup TBD)
    // Calculate expected voltage and current
    // Return success if closed loop error within 10%
}
pub fn test_tether_bias() -> (Status, Status){
    // Set cathode voltage
    // Read cathode voltage 
    // Read cathode current (setup TBD)
    // Calculate expected voltage and current
    // Return success if closed loop error within 10%
}
pub fn test_repeller() -> Status{
    // Read repeller voltage
    // Calculate expected voltage
    // Return success if error within 10%
}
pub fn test_heater() -> Status {
    // Manually set repeller voltage
    // Read repeller voltage
    // Return success if 
}
pub fn test_aperture() -> Status {
    // Set aperture current (setup TBD)
    // Calculate expected current
}
pub fn test_pinpuller_current_sensor() -> Status {
    // Also test the pinpuller current sensor
}

enum Status{
    Success(f32), // accuracy error in %
    Inaccurate(f32),
    NotWorking(f32),
}