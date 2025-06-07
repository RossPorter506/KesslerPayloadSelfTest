use embedded_hal::{digital::v2::{OutputPin, InputPin}, timer::CountDown};
use msp430fr2355::{E_USCI_A1, TB0};
use msp430fr2x5x_hal::serial::{SerialUsci, Rx};
use msp430fr2x5x_hal::timer::Timer;
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use nb::block;
use ufmt::{uWrite, uwrite, uwriteln};
use void::ResultVoidExt;

use crate::{dbg_println, delay_cycles, println};
use crate::payload::{Payload, PayloadState, PayloadState::*, HeaterState, HeaterState::*, SwitchState};
use crate::serial::{SerialWriter, wait_for_any_packet};
#[allow(unused_imports)]
use crate::{spi::{*, SckPolarity::*, SckPhase::SampleFirstEdge}, adc::*, digipot::*, dac::*};
#[allow(unused_imports)]
use crate::pcb_mapping::{pin_name_types::*, sensor_locations::*, power_supply_limits::*, power_supply_locations::*, peripheral_vcc_values::*, *};
use crate::serial::{read_num, TextColours::*};
use fixed::{self, FixedI64};
type Fxd = FixedI64::<32>;

use crate::testing::{calculate_performance_result, calculate_rpd, in_place_average, hvdc_mock,heater_mock,pinpuller_mock, PerformanceResult};

const CELCIUS_TO_KELVIN_OFFSET: u16 = 273;

pub fn emission_sensing(
    expected_heater_voltage_mv: u32,
    expected_tb_voltage_mv: u32,
    expected_co_voltage_mv: u32,
    payload: &mut Payload<{PayloadOn}, {HeaterOn}>){

    // Compare heater voltage AND current against expected values
    dbg_println!("");
    for sensor_result in compare_heater(expected_heater_voltage_mv, payload).iter(){
        println!("{}", sensor_result);
    }

    // Compare tether bias and cathode offset voltages against expected values
    let fn_arr          = [compare_cathode_offset, compare_tether_bias];
    let expected_values = [expected_co_voltage_mv, expected_tb_voltage_mv];
    for (sensor_fn, expected_voltage) in fn_arr.iter().zip(expected_values) {
        dbg_println!("");
        let result = sensor_fn(expected_voltage, payload);
        println!("{}", result);
    }

    // We don't have a good idea of what these *should* be, so just print out their value
    dbg_println!("");
    measure_aperture_current(payload);
    measure_repeller_voltage(payload);
    print_temperatures(payload);
}

pub fn deployment_sensing(payload: &mut Payload<{PayloadOff}, {HeaterOff}>) {
    println!("{}", compare_pinpuller_current(payload));
    print_temperatures(payload);
}

pub fn payload_off_sensing(payload: &mut Payload<{PayloadOff}, {HeaterOff}>) {
    print_temperatures(payload);
}

pub fn print_temperatures<const DONTCARE1:PayloadState, const DONTCARE2:HeaterState>(payload: &mut Payload<{DONTCARE1}, {DONTCARE2}>){

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

    for (n, (sensor, name)) in TEMP_SENSORS.iter().enumerate() {    
        let tempr = payload.get_temperature_kelvin(sensor) as i16;
        println!("{}: {}", name, tempr - (CELCIUS_TO_KELVIN_OFFSET as i16));     
    }
    println!("");
}

fn compare_hvdc_supply<const DONTCARE: HeaterState>(
    measure_voltage_fn: &dyn Fn(&mut Payload<{PayloadOn}, DONTCARE>) -> i32,
    measure_current_fn: &dyn Fn(&mut Payload<{PayloadOn}, DONTCARE>) -> i32,
    expected_voltage_mv: u32,
    payload: &mut Payload<{PayloadOn}, DONTCARE>) -> Fxd {
    
    const SENSE_RESISTANCE: u32 = 1; // Both supplies use the same sense resistor value
        
    // Read voltage, current
    let measured_voltage_mv = measure_voltage_fn(payload);
    let measured_current_ua = measure_current_fn(payload);
    dbg_println!("Measured output voltage: {}mV", measured_voltage_mv);
    dbg_println!("Measured output current: {}uA", measured_current_ua);

    // Calculate expected voltage and current
    let expected_voltage_mv: i32 = expected_voltage_mv as i32;

    dbg_println!("Expected output voltage: {}mV", expected_voltage_mv);

    let voltage_accuracy = calculate_rpd(measured_voltage_mv, expected_voltage_mv);
    
    voltage_accuracy
}

pub fn compare_cathode_offset<const DONTCARE: HeaterState>(
    expected_voltage_mv: u32,
    payload: &mut Payload<{PayloadOn}, DONTCARE>) -> PerformanceResult<'_> {

    let voltage_accuracy = self::compare_hvdc_supply(
            &Payload::get_cathode_offset_voltage_millivolts, 
            &Payload::get_cathode_offset_current_microamps, 
            expected_voltage_mv,
            payload);

    let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
    voltage_result
}

pub fn compare_tether_bias<const DONTCARE: HeaterState>(
    expected_voltage_mv: u32,
    payload: &mut Payload<{PayloadOn}, DONTCARE>) -> PerformanceResult<'_> { 

    let voltage_accuracy = self::compare_hvdc_supply(
            &Payload::get_tether_bias_voltage_millivolts, 
            &Payload::get_tether_bias_current_microamps, 
            expected_voltage_mv,
            payload);

    let voltage_result = calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
    voltage_result
}

pub fn compare_heater(
    expected_voltage_mv: u32,
    payload: &mut Payload<{PayloadOn}, {HeaterOn}>) -> [PerformanceResult<'_>; 2] {

    // Read voltage, current
    let heater_voltage_mv = payload.get_heater_voltage_millivolts();
    dbg_println!("Read voltage as: {}mV", heater_voltage_mv);
    let heater_current_ma = payload.get_heater_current_milliamps();
    dbg_println!("Read current as: {}mA", heater_current_ma);

    // Calculate expected voltage and current
    let expected_voltage_mv = expected_voltage_mv as u16;
    let expected_current_ma: i16 = (expected_voltage_mv as u32 * 1000 / heater_mock::CIRCUIT_RESISTANCE_MOHMS as u32)
            .min(heater_mock::POWER_LIMITED_MAX_CURRENT_MA.to_num()) as i16;
    dbg_println!("Expected current is: {}mA", expected_current_ma);

    // RPD and accuracy calculations
    let voltage_rpd = calculate_rpd(heater_voltage_mv as i32, expected_voltage_mv as i32);
    let current_rpd = calculate_rpd(heater_current_ma as i32, expected_current_ma as i32);
    

    let voltage_result = calculate_performance_result("Heater voltage", voltage_rpd, 5, 20);
    let current_result = calculate_performance_result("Heater current", current_rpd, 5, 20);

    [voltage_result, current_result]
}

pub fn measure_repeller_voltage<const DONTCARE: HeaterState>(
    payload: &mut Payload<{PayloadOn}, DONTCARE>,) {
    
    // Read voltage
    let repeller_voltage_mv = payload.get_repeller_voltage_millivolts();
    println!("Repeller voltage measured as: {}mV", repeller_voltage_mv);

    // Calculate expected voltage/current
    // Do we actually know what the repeller voltage should be?
    //let voltage_rpd = calculate_rpd(repeller_voltage_mv, expected_voltage_mv as i32);
    
    //calculate_performance_result("Repeller voltage", voltage_rpd, 5, 20)
}

pub fn compare_pinpuller_current<const DONTCARE1: PayloadState, const DONTCARE2:HeaterState>(
    payload: &mut Payload<DONTCARE1, DONTCARE2>) -> PerformanceResult<'_>{

    let measured_current = payload.get_pinpuller_current_milliamps();
    dbg_println!("Pinpuller current measured as: {}mA", measured_current);
    let accuracy = calculate_rpd(measured_current as i32, pinpuller_mock::EXPECTED_ON_CURRENT.to_num());

    calculate_performance_result("Pinpuller current sense",  accuracy,  5, 20)
}

pub fn measure_aperture_current<const DONTCARE1: PayloadState, const DONTCARE2:HeaterState>(
    payload: &mut Payload<DONTCARE1, DONTCARE2>) {

    let measured_current = payload.get_aperture_current_microamps();
    println!("Aperture current measured as: {}uA", measured_current);
}

pub fn aperture_current_sense_validation(mut serial_writer: SerialWriter<E_USCI_A1>, payload: &mut Payload<{PayloadOn}, {HeaterOn}>, mut payload_spi_controller: PayloadSPIController) {
    // Name of test
    println!("========== VACUUM CHAMBER - APERTURE CURRENT SENSE VALIDATION FIRMWARE ==========");
    println!("");
    delay_cycles(2_000_000);

    // Automated performance test to ensure setup is correct
    println!("========== AUTOMATED PERFORMANCE TEST START ==========");
    println!("{}", crate::testing::AutomatedPerformanceTests::test_cathode_offset_voltage(payload));

    println!("========== AUTOMATED PERFORMANCE TEST COMPLETE ==========");
    println!("");
    delay_cycles(2_000_000);

    // // Warning to switch off power supply if the test specimen is not in vacuum
    println!("========== If the vacuum chamber is not depressurised, please turn off power supply now ==========");
    println!("");
    delay_cycles(5_000_000);
    println!("========== The vacuum test will initiate in T-: ==========");
    
    for i in 0..20{
        println!("========== {} s ==========", 20-i);
        delay_cycles(1_000_000);
    }        

    // Perform electron emission test
    crate::testing::AutomatedPerformanceTests::test_aperture_current_sensor(payload, &mut payload_spi_controller,&mut serial_writer);
    
    println!("========== TEST COMPLETE ==========");
}


pub fn tvac_test(payload: Payload<{PayloadOff}, {HeaterOff}>) -> ! {
    println!("==========TVAC TEST FIRMWARE==========");
    delay_cycles(2_000_000);

    let mut payload = payload.into_enabled_payload().into_enabled_heater();
    crate::testing::AutomatedPerformanceTests::full_system_test(&mut payload);
    let mut payload = payload.into_disabled_heater().into_disabled_payload();

    payload.led_pins.green_led.set_high().ok();

    payload.timer.start(32768u16);
    let mut sec_elapsed_phase:u32 = 0;
    let mut sec_elapsed_total:u32 = 0;

    //  To avoid 'use of moved value' as we mutate the type of payload between loop iterations(?), we make a second variable to store payload here between loops. 
    let mut payload_off: Option<Payload<{PayloadOff}, {HeaterOff}>> = Some(payload);

    loop{
        // ------------------------------------------------------------------------
        // -------------------------- Payload Off ---------------------------------
        // ------------------------------------------------------------------------
        println!("ENTERING PAYLOAD-OFF PHASE");
        if let Some(payload) = payload_off.as_mut() { 
            for _ in 0..45*60{
                // LEAVE PAYLOAD OFF FOR 45 MINUTES
                block!(payload.timer.wait()).void_unwrap();
                sec_elapsed_phase += 1;
                sec_elapsed_total += 1;
                println!("{} seconds elapsed in the current phase", sec_elapsed_phase);
                println!("{} seconds elapsed in the total test", sec_elapsed_total);
                payload_off_sensing(payload);
                
            }

            println!("");
            sec_elapsed_phase = 0;

            // ------------------------------------------------------------------------
            // ----------------------  Pinpuller activation ---------------------------
            // ------------------------------------------------------------------------
            println!("ENTERING PINPULLER ACTIVATION PHASE");
            // activate pinpuller and LMS
        
            payload.pinpuller_pins.burn_wire_1.set_high().ok();
            payload.lms_control_pins.lms_led_enable.set_high().ok();
            payload.lms_control_pins.lms_receiver_enable.set_high().ok();
            payload.led_pins.yellow_led.set_high().ok();

            for _ in 0..60{           
                // LEAVE PINPULLER ON FOR 60 SECONDS
                block!(payload.timer.wait()).void_unwrap();
                sec_elapsed_phase += 1;
                sec_elapsed_total += 1;
                println!("{} seconds elapsed in the current phase", sec_elapsed_phase);
                println!("{} seconds elapsed in the total test", sec_elapsed_total);
                
                deployment_sensing(payload);
            }

            // disable pinpuller and LMS
            payload.pinpuller_pins.burn_wire_1.set_low().ok();
            payload.lms_control_pins.lms_led_enable.set_low().ok();
            payload.lms_control_pins.lms_receiver_enable.set_low().ok();
        }
        
        println!("");
        sec_elapsed_phase = 0;

        // ------------------------------------------------------------------------
        // ---------------------------  Emission  ---------------------------------
        // ------------------------------------------------------------------------
        println!("ENTERING EMISSION PHASE");
        // Payload On activated for 44 minutes
        let mut payload = payload_off.unwrap().into_enabled_payload().into_enabled_heater();
        
        payload.set_cathode_offset_switch(SwitchState::Connected);
        payload.set_tether_bias_switch(SwitchState::Connected);
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS);
        payload.set_tether_bias_voltage(TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS);
        payload.set_heater_voltage(3160);
        payload.led_pins.red_led.set_high().ok();

        for _ in 0..44*60{
            // ENTER CODE TO READ SENSORS FOR 44 MINUTES
            block!(payload.timer.wait()).void_unwrap();
            sec_elapsed_phase += 1;
            sec_elapsed_total += 1;
            println!("{} seconds elapsed in the current phase", sec_elapsed_phase);
            println!("{} seconds elapsed in the total test", sec_elapsed_total);
            emission_sensing(3160, TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS, CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS, 
                &mut payload)
        }

        payload.set_cathode_offset_switch(SwitchState::Disconnected);
        payload.set_tether_bias_switch(SwitchState::Disconnected);
        payload.led_pins.yellow_led.set_low().ok();
        payload.led_pins.red_led.set_low().ok();
        payload_off = Some(payload.into_disabled_heater().into_disabled_payload());

        println!("");
        sec_elapsed_phase = 0;
    }
}