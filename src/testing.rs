use embedded_hal::digital::v2::{OutputPin, InputPin};
use embedded_hal::serial::Read;
use fixed::types::extra::U32;
use msp430fr2x5x_hal::serial::{SerialUsci, Rx};
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use replace_with::replace_with;
use ufmt::{uWrite, uwrite, uwriteln};

use crate::delay_cycles;
use crate::sensors::{PayloadController, PayloadOn, PayloadState, PayloadOff, HeaterState, HeaterOn, SwitchState};
use crate::serial::SerialWriter;
#[allow(unused_imports)]
use crate::{spi::*, adc::*, digipot::*, dac::*};
#[allow(unused_imports)]
use crate::pcb_mapping_v5::{pin_name_types::*, sensor_locations::*, power_supply_limits::*, power_supply_locations::*, peripheral_vcc_values::*, *};
use fixed::{self, FixedI64};
// Tests that (potentially after some setup - devices, jumpers, shorts, etc.) can be done without user intervention
// These tests often rely on a sensor and an actuator together, so they test multiple components at once
// Functional (pass/fail) tests
pub struct AutomatedFunctionalTests {}
impl AutomatedFunctionalTests{
    pub fn full_system_test<USCI:SerialUsci>(
            payload: &mut PayloadController<PayloadOn, HeaterOn>, 
            pinpuller_pins: &mut PinpullerActivationPins, 
            lms_pins: &mut TetherLMSPins,
            spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>, 
            serial: &mut SerialWriter<USCI>){

        uwriteln!(serial, "==== Functional Tests Start ====").ok();
        for adc_test_fn in [Self::tether_adc_functional_test, Self::temperature_adc_functional_test, Self::misc_adc_functional_test].iter(){
            uwriteln!(serial, "{}", adc_test_fn(payload, spi_bus)).ok();
        }

        for pinpuller_lane in Self::pinpuller_functional_test(pinpuller_pins, payload, spi_bus).iter() {
            uwriteln!(serial, "{}", pinpuller_lane).ok();
        }

        uwriteln!(serial, "{}", Self::heater_functional_test(payload, spi_bus)).ok();

        for lms_channel in Self::lms_functional_test(payload, lms_pins, spi_bus).iter(){
            uwriteln!(serial, "{}", lms_channel).ok();
        }
    }
    // Internal function to reduce code duplication
    fn test_adc_functional<CsPin: ADCCSPin, SENSOR:ADCSensor>(  
            adc: &mut ADC<CsPin, SENSOR>, 
            spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFirstEdge>,
            wanted_channel: ADCChannel) -> bool {
        let data_packet = (wanted_channel as u32) << (NUM_CYCLES_FOR_TWO_READINGS-1 - 2 - 2); // see adc.rs
        let result = spi_bus.send_and_receive(NUM_CYCLES_FOR_TWO_READINGS, data_packet, &mut adc.cs_pin);
        let zeroes = result & 0xF000_F000;

        zeroes == 0
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: Isolated 5V supply, tether ADC, isolators
    pub fn tether_adc_functional_test<'a, DONTCARE:HeaterState>(
            payload: &'a mut PayloadController<PayloadOn, DONTCARE>, 
            spi_bus: &'a mut impl PayloadSPI<IdleHigh, SampleFirstEdge>) -> SensorResult<'a> {
        let result = Self::test_adc_functional(&mut payload.tether_adc, spi_bus, ADCChannel::IN7);
        SensorResult { name: "Tether ADC", result }
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: temperature ADC
    pub fn temperature_adc_functional_test<'a, DONTCARE1: PayloadState, DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            spi_bus: &'a mut impl PayloadSPI<IdleHigh, SampleFirstEdge>) -> SensorResult<'a> {
        let result = Self::test_adc_functional(&mut payload.temperature_adc, spi_bus, ADCChannel::IN7);
        SensorResult { name: "Temperature ADC", result }
    }

    // Ask to read channel 7.
    // Return success if SPI packet valid
    // Dependencies: misc ADC
    pub fn misc_adc_functional_test<'a, DONTCARE1: PayloadState, DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            spi_bus: &'a mut impl PayloadSPI<IdleHigh, SampleFirstEdge>) -> SensorResult<'a> {
        let result =Self::test_adc_functional(&mut payload.misc_adc, spi_bus, ADCChannel::IN7);
        SensorResult { name: "Misc ADC", result }
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
    pub fn pinpuller_functional_test<'a, DONTCARE1: PayloadState, DONTCARE2:HeaterState>(   
            pins: &'a mut PinpullerActivationPins, 
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>,
            spi_bus: &'a mut impl PayloadSPI<IdleHigh, SampleFirstEdge>) -> [SensorResult<'a>; 4] {
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

    // Dependencies: Tether ADC, digipot, isolated 5V supply, isolated 12V supply, heater step-down regulator, signal processing circuitry, isolators
    pub fn heater_functional_test<'a>(
            payload: &'a mut PayloadController<PayloadOn, HeaterOn>, 
            spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> SensorResult<'a> {
        // Because we're alternating between talking with the ADCs (which expect IdleHigh) and the digipot (which expects IdleLow), we need to temporarily move outside a borrowed value.
        // To do this we need to temporarily take ownership of the bus to change it's typestate using the 'replace with' function.
        // Alternative is to own the SPI bus rather than take a &mut, then return it alongside the result. Neither option is really that clean.

        let mut min_voltage_mv: u16 = 0;
        let mut max_voltage_mv: u16 = 0;

        // Configure SPI bus for digipot and set minimum voltage
        replace_with(spi_bus, default_payload_spi_bus, |spi_bus| {
            // Set heater to max
            let mut spi_bus = spi_bus.into_idle_low(); //configure bus for digipot
            payload.set_heater_voltage(HEATER_MAX_VOLTAGE_MILLIVOLTS, &mut spi_bus); // set voltage
            delay_cycles(1000);

            // Read voltage
            let mut spi_bus = spi_bus.into_idle_high(); 
            max_voltage_mv = payload.get_heater_voltage_millivolts(&mut spi_bus);
            
            // Set heater to min
            let mut spi_bus = spi_bus.into_idle_low(); //configure bus for digipot
            payload.set_heater_voltage(HEATER_MIN_VOLTAGE_MILLIVOLTS, &mut spi_bus); // set voltage
            delay_cycles(1000);

            // Read voltage
            let mut spi_bus = spi_bus.into_idle_high(); 
            min_voltage_mv = payload.get_heater_voltage_millivolts(&mut spi_bus);
            
            spi_bus // return bus
        });

        SensorResult{name: "Heater", result: (min_voltage_mv < 50 && max_voltage_mv > 10_000) }
    }
    // Dependencies: LMS power switches, misc ADC, LMS LEDs, LMS receivers
    // Setup: Connect LMS board, test in a room with minimal (or at least uniform) IR interference. 
    pub fn lms_functional_test<'a, DONTCARE1: PayloadState, DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            lms_control: &'a mut TetherLMSPins, 
            spi_bus: &'a mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> [SensorResult<'a>;3] {
        let mut ambient_counts: [u16; 3] = [0; 3];
        let mut on_counts: [u16; 3] = [0; 3];

        // Enable phototransistors
        lms_control.lms_receiver_enable.set_high().ok();

        // Record max voltage/light value
        for (n, sensor) in [LMS_RECEIVER_1_SENSOR, LMS_RECEIVER_2_SENSOR, LMS_RECEIVER_2_SENSOR].iter().enumerate() {
            ambient_counts[n] = payload.misc_adc.read_count_from(&sensor, spi_bus);
        }

        // Enable LEDs
        lms_control.lms_led_enable.set_high().ok();

        // Record max voltage/light value
        for (n, sensor) in [LMS_RECEIVER_1_SENSOR, LMS_RECEIVER_2_SENSOR, LMS_RECEIVER_2_SENSOR].iter().enumerate() {
            on_counts[n] = payload.misc_adc.read_count_from(&sensor, spi_bus);
        }

        lms_control.lms_receiver_enable.set_low().ok();
        lms_control.lms_led_enable.set_low().ok();

        [SensorResult{name: "Length measurement system 1", result: (on_counts[0] > 2*ambient_counts[0])}, 
         SensorResult{name: "Length measurement system 2", result: (on_counts[1] > 2*ambient_counts[1])}, 
         SensorResult{name: "Length measurement system 3", result: (on_counts[2] > 2*ambient_counts[2])}]
    }

}

// DO NOT USE OUTSIDE OF 'replace_with'! WILL panic if called!
// Make sure your replace_with call is panic-free!!
#[allow(unreachable_code)]
fn default_payload_spi_bus() -> PayloadSPIBitBang<IdleHigh, SampleFirstEdge>{
    unreachable!(); // This will panic.
    let periph = msp430fr2355::Peripherals::take().unwrap(); //so will this 
    let pmm = Pmm::new(periph.PMM);
    let port4 = Batch::new(periph.P4).split(&pmm);
    PayloadSPIBitBangConfig::new_from_pins(   
        port4.pin7.pullup(),
        port4.pin6.to_output(),
        port4.pin5.to_output(),)
        .sck_idle_high()
        .sample_on_first_edge()
        .create()
}

// Relative percent difference. 0 means spot-on, 1 means measured is far larger than actual, -1 means measured is far smaller than actual
fn calculate_rpd(measured:i32, actual: i32) -> FixedI64<U32> {
    if actual == 0 && measured == 0{
        return FixedI64::<U32>::from(0);
    }
    let actual = FixedI64::<U32>::from(actual);
    let measured = FixedI64::<U32>::from(measured);

    (measured - actual) / (measured.abs() + actual.abs())
}
fn average<T: Copy + Into<f64>>(arr: &[T]) -> f64 {
    let mut cumulative_avg: f64 = 0.0;
    for (i, num) in arr.iter().enumerate() {
        cumulative_avg += ((*num).into() - cumulative_avg) / ((i+1) as f64);
    }
    cumulative_avg
}

fn in_place_average(acc: FixedI64<U32>, new: FixedI64<U32>, n: u16) -> FixedI64<U32>{
    acc + ((new - acc) / FixedI64::<U32>::from(n+1))
} 
fn calculate_performance_result<'a, 'b>(name: &'a str, rpd: FixedI64<U32>, success_threshhold: FixedI64<U32>, inaccurate_threshhold: FixedI64<U32>) -> PerformanceResult<'a> {

    let performance = match rpd.abs() {
        x if x < success_threshhold    => Performance::Nominal,
        x if x < inaccurate_threshhold => Performance::Inaccurate,
        _ => Performance::NotWorking};
    PerformanceResult{name, performance, accuracy: rpd}
}

fn min<T: PartialOrd>(a:T, b:T) -> T {
    if a >= b {b} else {a}
}

// Accuracy-based tests
pub struct AutomatedPerformanceTests {}
impl AutomatedPerformanceTests{
    pub fn full_system_test<USCI:SerialUsci>(
            payload: &mut PayloadController<PayloadOn, HeaterOn>, 
            pinpuller_pins: &mut PinpullerActivationPins,
            spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>, 
            serial: &mut SerialWriter<USCI>){
        uwriteln!(serial, "==== Performance Tests Start ====").ok();
        // Each of these three fn's takes the same arguments and both return a voltage and current result
        let fn_arr = [Self::test_cathode_offset, Self::test_tether_bias, Self::test_heater];
        for sensor_fn in fn_arr.iter(){
            for sensor_result in sensor_fn(payload, spi_bus).iter(){
                uwriteln!(serial, "{}", sensor_result).ok();
            }
        }

        uwriteln!(serial, "{}", Self::test_pinpuller_current_sensor(payload, pinpuller_pins, spi_bus)).ok();
    }
    // Dependencies: Isolated 5V supply, tether ADC, DAC, cathode offset supply, signal processing circuitry, isolators
    // Setup: Place a 100k resistor between exterior and cathode-
    pub fn test_cathode_offset<'a, DONTCARE:HeaterState>(
            payload: &'a mut PayloadController<PayloadOn, DONTCARE>, 
            spi_bus: &'a mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> [PerformanceResult<'a>; 2] {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: FixedI64<U32> = FixedI64::ZERO;
        let mut current_accuracy: FixedI64<U32> = FixedI64::ZERO;

        payload.set_cathode_offset_switch(SwitchState::Connected); // connect to exterior
        for (i, output_percentage) in (0..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv = ((CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS - CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS) * output_percentage) / 100;

            // Set cathode voltage
            //replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low();
                payload.set_cathode_offset_voltage(output_voltage_mv, &mut spi_bus_);
                spi_bus_.into_idle_high()
            });
            delay_cycles(10000); //settling time
            
            // Read cathode voltage, current
            let cathode_offset_voltage_mv = payload.get_cathode_offset_voltage_millivolts(spi_bus);
            let cathode_offset_current_ua = payload.get_cathode_offset_current_microamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = output_voltage_mv as i32;
            let expected_current_ua: i32 = 1000 * (output_voltage_mv / (TEST_RESISTANCE + TETHER_SENSE_RESISTANCE_OHMS)) as i32;

            voltage_accuracy = in_place_average(voltage_accuracy, calculate_rpd(cathode_offset_voltage_mv, expected_voltage_mv),i as u16);
            current_accuracy = in_place_average(current_accuracy, calculate_rpd(cathode_offset_current_ua, expected_current_ua),i as u16);
        }
        payload.set_cathode_offset_switch(SwitchState::Disconnected);

        let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100);
        let current_result = calculate_performance_result("Cathode offset current", current_accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100);
        [voltage_result, current_result]
    }
    // Almost identical code, feels bad man
    // Dependencies: isolated 5V supply, tether ADC, DAC, tether bias supply, signal processing circuitry, isolators
    // Setup: Place a 100k resistor between tether and cathode-
    pub fn test_tether_bias<'a, DONTCARE:HeaterState>(
            payload: &'a mut PayloadController<PayloadOn, DONTCARE>, 
            spi_bus: &'a mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> [PerformanceResult<'a>; 2] {
        const NUM_MEASUREMENTS: usize = 10;
        const TEST_RESISTANCE: u32 = 100_000;
        let mut voltage_accuracy: FixedI64<U32> = FixedI64::ZERO;
        let mut current_accuracy: FixedI64<U32> = FixedI64::ZERO;

        payload.set_tether_bias_switch(SwitchState::Connected); // connect to tether
        for (i, output_percentage) in (0..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage_mv = ((TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS - TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS) * output_percentage) / 100;

            // Set tether voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low();
                payload.set_tether_bias_voltage(output_voltage_mv, &mut spi_bus_);
                spi_bus_.into_idle_high()
            });
            delay_cycles(100_000); //settling time
            
            // Read tether voltage, current
            let tether_bias_voltage_mv = payload.get_tether_bias_voltage_millivolts(spi_bus);
            let tether_bias_current_ua = payload.get_tether_bias_current_microamps(spi_bus);
            
            // Calculate expected voltage and current
            let expected_voltage_mv: i32 = output_voltage_mv as i32;
            let expected_current_ma: i32 = (output_voltage_mv / (TEST_RESISTANCE + TETHER_SENSE_RESISTANCE_OHMS)) as i32;
            
            voltage_accuracy = in_place_average(voltage_accuracy, calculate_rpd(tether_bias_voltage_mv, expected_voltage_mv),i as u16);
            current_accuracy = in_place_average(current_accuracy, calculate_rpd(tether_bias_current_ua, expected_current_ma),i as u16);
        }
        payload.set_tether_bias_switch(SwitchState::Disconnected);

        let voltage_result = calculate_performance_result("Tether bias voltage", voltage_accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100);
        let current_result = calculate_performance_result("Tether bias current", current_accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100);
        [voltage_result, current_result]
    }
    // Generic version, couldn't get working due to overlapping borrows for function pointers
    /*fn test_generic_voltage_current(supply_max: u32, supply_min: u32, success_threshhold: f64, inaccurate_threshhold: f64,
                                    read_voltage_fn: &dyn Fn(&mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> i32,
                                    read_current_fn: &dyn Fn(&mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> i32,
                                    set_voltage_fn:  &dyn Fn(u32, &mut PayloadSPIBitBang<IdleLow, SampleFirstEdge>),
                                    calculate_current_fn: &dyn Fn(i32) -> i32,
                                    spi_bus: &mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> (PerformanceResult, PerformanceResult) {
        const NUM_MEASUREMENTS: usize = 10;
        let mut voltage_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];
        let mut current_accuracy_measurements: [f64;NUM_MEASUREMENTS] = [0.0; NUM_MEASUREMENTS];

        for (i, output_percentage) in (0..=100u8).step_by(NUM_MEASUREMENTS).enumerate() {
            let output_fraction =  output_percentage as f32 * 0.01;
            let output_voltage = ((supply_max - supply_min) as f32 * output_fraction) as u32;

            // Set voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low();
                set_voltage_fn(output_voltage, &mut spi_bus_);
                spi_bus_.into_idle_high()
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
    pub fn test_heater<'a>(
            payload: &'a mut PayloadController<PayloadOn, HeaterOn>, 
            spi_bus: &'a mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> [PerformanceResult<'a>; 2] {
        const NUM_MEASUREMENTS: usize = 10;
        let heater_resistance = FixedI64::<U32>::from(10) + FixedI64::<U32>::from(1) / 100; // heater resistance + shunt resistor
        let heater_max_power = FixedI64::<U32>::from(1); // TODO: Verify?
        let max_on_current_ma = FixedI64::<U32>::from(HEATER_MAX_VOLTAGE_MILLIVOLTS) / heater_resistance; 

        let power_limited_max_current_ma: u32 = 316;//1000 * sqrt((heater_max_power / heater_resistance).to_num());
        let mut voltage_accuracy: FixedI64<U32> = FixedI64::ZERO;
        let mut current_accuracy: FixedI64<U32> = FixedI64::ZERO;

        for (i, output_percentage) in (0..=100u32).step_by(100/NUM_MEASUREMENTS).enumerate() {
            let output_voltage: u16 = ((HEATER_MAX_VOLTAGE_MILLIVOLTS - HEATER_MIN_VOLTAGE_MILLIVOLTS) * output_percentage as u16)  / 100;

            // Set cathode voltage
            replace_with(spi_bus, default_payload_spi_bus, |spi_bus_| {
                let mut spi_bus_ = spi_bus_.into_idle_low();
                payload.set_heater_voltage(output_voltage, &mut spi_bus_);
                spi_bus_.into_idle_high()
            });
            delay_cycles(100_000); //settling time
            
            // Read voltage, current
            let heater_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
            let heater_current_ma = payload.get_heater_current_milliamps(spi_bus);

            // Calculate expected voltage and current
            let expected_voltage: u16 = output_voltage;
            let expected_current: i16 = power_limited_max_current_ma.min(max_on_current_ma.to_num::<u32>() * output_percentage / 100) as i16;

            voltage_accuracy = in_place_average(voltage_accuracy, calculate_rpd(heater_voltage_mv as i32, expected_voltage as i32), i as u16);
            current_accuracy = in_place_average(current_accuracy, calculate_rpd(heater_current_ma as i32, expected_current as i32), i as u16);
        }

        let voltage_result = calculate_performance_result("Heater voltage", voltage_accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100);
        let current_result = calculate_performance_result("Heater current", current_accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100);
        [voltage_result, current_result]
    }
    
    // Dependencies: Pinpuller, pinpuller current sensor, misc ADC, signal processing circuitry
    // Setup: Place 2 ohm (10W+) resistor between pinpuller pins. // TODO
    pub fn test_pinpuller_current_sensor<'a, DONTCARE1: PayloadState, DONTCARE2:HeaterState>(
            payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
            p_pins: &'a mut PinpullerActivationPins, 
            spi_bus: &'a mut PayloadSPIBitBang<IdleHigh, SampleFirstEdge>) -> PerformanceResult<'a> {
        const EXPECTED_OFF_CURRENT: u16 = 0;
        let mosfet_r_on_resistance: FixedI64<U32> = FixedI64::<U32>::from(3)/100; // Verify(?)
        let pinpuller_mock_resistance: FixedI64<U32> = FixedI64::<U32>::from(2);
        let sense_resistance: FixedI64<U32> = FixedI64::<U32>::from(4)/10;
        const NUM_PINS: usize = 4;
        let expected_on_current: u16 = (FixedI64::<U32>::from(PINPULLER_VOLTAGE_MILLIVOLTS) / (pinpuller_mock_resistance + sense_resistance + mosfet_r_on_resistance*2)).to_num();
        
        let mut accuracy: FixedI64<U32> = FixedI64::ZERO;
        //let mut accuracy_measurements: [f32; NUM_PINS+1] = [0.0; NUM_PINS+1];

        accuracy = in_place_average(accuracy, calculate_rpd(payload.get_pinpuller_current_milliamps(spi_bus) as i32, 0),0); 

        // For each pin, activate the pinpuller through that channel and measure the current
        let mut pin_list: [&mut dyn OutputPin<Error = void::Void>; NUM_PINS] = [&mut p_pins.burn_wire_1, &mut p_pins.burn_wire_1_backup, &mut p_pins.burn_wire_2, &mut p_pins.burn_wire_2_backup];
        for (n, pin) in pin_list.iter_mut().enumerate() {
            pin.set_high().ok();
            accuracy = in_place_average(accuracy, calculate_rpd(payload.get_pinpuller_current_milliamps(spi_bus) as i32, expected_on_current as i32), (n+1) as u16);
            pin.set_low().ok();
            delay_cycles(1000);
        }

        calculate_performance_result("Pinpuller current sense", accuracy, FixedI64::<U32>::from(5)/100, FixedI64::<U32>::from(20)/100)
    }    
}

// Block until we receive any packet over serial
fn wait_for_any_packet<USCI: SerialUsci>(serial_reader: &mut Rx<USCI>) -> u8{
    loop {
        match serial_reader.read() {
            Ok(chr) => return chr,
            _ => (),
        }
    }
}
// Block until we receive the specified character
fn wait_for_character<USCI: SerialUsci>(wanted_char: u8, serial_reader: &mut Rx<USCI>) {
    while wait_for_any_packet(serial_reader) != wanted_char {}
}
fn wait_for_string<USCI: SerialUsci>(wanted_str: &str, serial_reader: &mut Rx<USCI>) {
    for chr in wanted_str.as_bytes(){
        wait_for_character(*chr, serial_reader);
    }
}

// Tests that require human intervention during the test
// Functional (pass/fail) tests
pub struct ManualFunctionalTests{}
impl ManualFunctionalTests{
    // Dependencies: endmass switches
    pub fn endmass_switches_functional_test<'a, USCI: SerialUsci>(
            pins: &mut DeploySensePins,
            serial_writer: &'a mut SerialWriter<USCI>, 
            serial_reader: &'a mut Rx<USCI>) -> [SensorResult<'a>; 2] {


        uwriteln!(serial_writer, "Depress switches").ok();
        
        wait_for_any_packet(serial_reader);

        let depressed_states: [bool; 2] = [pins.endmass_sense_1.is_high().unwrap(), pins.endmass_sense_1.is_high().unwrap()];

        uwriteln!(serial_writer, "Release switches").ok();
        
        wait_for_any_packet(serial_reader);

        let released_states: [bool; 2] = [pins.endmass_sense_1.is_high().unwrap(), pins.endmass_sense_1.is_high().unwrap()];

        [SensorResult {name: "Endmass switch 1", result: (depressed_states[0] != released_states[0])},
         SensorResult {name: "Endmass switch 2", result: (depressed_states[1] != released_states[1])}]
    }
    /*
    // Dependencies: pinpuller
    pub fn pinpuller_functional_test() -> [SensorResult; 4] {
        // Enable each of the four redundant lines.
        // Manually check resistance(?) across pinpuller pins
        todo!();
    }
    // Dependencies: LMS power switches
    pub fn lms_power_switch_functional_test() -> [SensorResult; 2] {
        // Enable LMS LED EN
        // Measure resistance between J1 pin 2A/B and GND
        // Enable LMS Receiver EN
        // Manually measure resistance between J1 pin 3A/B and GND
        // Query user for resistance 
        // Return true if resistance less than 1-10 ohms
        todo!();
    }
    // Dependencies: pinpuller
    pub fn pinpuller_sense_functional_test() -> SensorResult {
        // Read pinpuller sense lines
        // Short pinpuller sense lines
        // Read pinpuller sense lines again
        // Return true if different
        todo!();
    }*/
}

const TEMPERATURE_SENSOR_SUCCESS: u8 = 5; // within 5% of true value, etc
const TEMPERATURE_SENSOR_INACCURATE: u8 = 20;
fn test_temperature_sensors_against_known_temp<'a, DONTCARE:PayloadState, ALSODONTCARE:HeaterState, USCI:SerialUsci>(
        room_temp_k: u16,
        payload: &'a mut PayloadController<DONTCARE, ALSODONTCARE>,
        serial_writer: &'a mut SerialWriter<USCI>,
        serial_reader: &'a mut Rx<USCI>, 
        spi_bus: &'a mut impl PayloadSPI<IdleHigh, SampleFirstEdge>) -> [PerformanceResult<'static>; 8]{
    
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
    for (n, temp_sensor) in TEMP_SENSORS.iter().enumerate() {
        let tempr = payload.get_temperature_kelvin(&temp_sensor.0, spi_bus);
        let accuracy = calculate_rpd(tempr as i32, room_temp_k as i32);
        output_arr[n] = calculate_performance_result(temp_sensor.1, accuracy, FixedI64::<U32>::from(TEMPERATURE_SENSOR_SUCCESS)/100, FixedI64::<U32>::from(TEMPERATURE_SENSOR_INACCURATE)/100)
    }

    output_arr
}

const ASCII_ZERO: u8 = 48;
const ASCII_NINE: u8 = 57;
// Accuracy-based tests
pub struct ManualPerformanceTests{}
impl ManualPerformanceTests{
    /*
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
    }*/
    fn is_ascii_number(c: u8) -> bool {
        c >= ASCII_ZERO && c <= ASCII_NINE
    }
    // Get room temp from user
    fn query_room_temp<USCI:SerialUsci>(serial_writer: &mut SerialWriter<USCI>, serial_reader: &mut Rx<USCI>) -> u16 {
        loop {
            uwriteln!(serial_writer, "Enter current temp (>0, two digits, in celcius)").ok();
            
            let tens_chr = wait_for_any_packet(serial_reader);
            let ones_chr = wait_for_any_packet(serial_reader);
            if Self::is_ascii_number(tens_chr) && Self::is_ascii_number(ones_chr) {
                let ones = ones_chr - 48;
                let tens = tens_chr - 48;
                return 10*(tens as u16) + (ones as u16) + 274;
            }
            else{
                uwriteln!(serial_writer, "Invalid temperature").ok();
            }
        };
    }
    pub fn two_point_test_temperature_sensor_test<'a, USCI:SerialUsci, DONTCARE:HeaterState>( 
            payload: &'a mut PayloadController<PayloadOff, DONTCARE>, // Minimise heat generation
            serial_writer: &'a mut SerialWriter<USCI>,
            serial_reader: &'a mut Rx<USCI>, 
            spi_bus: &'a mut impl PayloadSPI<IdleHigh, SampleFirstEdge>) -> [PerformanceResult<'a>; 8]{


        let mut room_temp_k: u16 = Self::query_room_temp(serial_writer, serial_reader);

        let arr1 = test_temperature_sensors_against_known_temp(room_temp_k, payload, serial_writer, serial_reader, spi_bus);

        room_temp_k = Self::query_room_temp(serial_writer, serial_reader);

        let arr2 = test_temperature_sensors_against_known_temp(room_temp_k, payload, serial_writer, serial_reader, spi_bus);

        let mut result_arr: [PerformanceResult; 8] = [PerformanceResult::default(); 8];

        for (n, (result1, result2)) in arr1.iter().zip(arr2.iter()).enumerate(){
            let accuracy = (result1.accuracy + result2.accuracy) / 2;
            result_arr[n] = calculate_performance_result(result1.name, accuracy, FixedI64::<U32>::from(TEMPERATURE_SENSOR_SUCCESS)/100, FixedI64::<U32>::from(TEMPERATURE_SENSOR_INACCURATE)/100)
        }
        result_arr
    }
    /*
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
    }*/
}

pub struct SensorResult<'a> {
    name: &'a str,
    result: bool,
}
impl ufmt::uDisplay for SensorResult<'_> {
    fn fmt<W: uWrite + ?Sized>(&self, f: &mut ufmt::Formatter<W>) -> Result<(), W::Error> {
        let result = match self.result {
            true => " OK ",
            false => "FAIL"};

        uwrite!(f, "[{}] {}", result, self.name).ok();
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct PerformanceResult<'a>{
    name: &'a str, 
    performance: Performance,
    accuracy: FixedI64<U32>, // relative percent difference / 2
}
impl PerformanceResult<'_>{
    fn default<'a>()-> PerformanceResult<'a> {
        PerformanceResult{name: "", performance: Performance::NotWorking, accuracy: FixedI64::<U32>::from(0)}
    }
}
#[derive(Copy, Clone)]
pub enum Performance {
    Nominal,
    Inaccurate,
    NotWorking,
}
impl ufmt::uDisplay for PerformanceResult<'_> {
    fn fmt<W: uWrite + ?Sized>(&self, f: &mut ufmt::Formatter<W>) -> Result<(), W::Error> {
        let result = match self.performance {
            Performance::Nominal    => " OK ",
            Performance::Inaccurate => "INAC",
            Performance::NotWorking => "FAIL"};
        let percent_acc: u32 = (self.accuracy*100).to_num();
        let fractional_percent: u32 = (self.accuracy*10000).to_num::<u32>() - percent_acc*100;
        uwrite!(f, "[{}] {}, {}.{}%", result, self.name, percent_acc, fractional_percent).ok();
        Ok(())
    }
}
