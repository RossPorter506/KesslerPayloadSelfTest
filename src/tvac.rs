use embedded_hal::digital::v2::{OutputPin, InputPin};
use msp430fr2x5x_hal::serial::{SerialUsci, Rx};
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use sensor_equations::aperture_current_sensor_eq;
use ufmt::{uWrite, uwrite, uwriteln};

use crate::delay_cycles;
use crate::payload::{PayloadController, PayloadState, PayloadState::*, HeaterState, HeaterState::*, SwitchState};
use crate::serial::{SerialWriter, wait_for_any_packet};
#[allow(unused_imports)]
use crate::{spi::{*, SckPolarity::*, SckPhase::SampleFirstEdge}, adc::*, digipot::*, dac::*};
#[allow(unused_imports)]
use crate::pcb_mapping::{pin_name_types::*, sensor_locations::*, power_supply_limits::*, power_supply_locations::*, peripheral_vcc_values::*, *};
use crate::serial::{read_num, TextColours::*};
use crate::{dbg_uwriteln, uwrite_coloured};
use fixed::{self, FixedI64};
type Fxd = FixedI64::<32>;

use crate::testing::{calculate_performance_result, calculate_rpd, in_place_average, hvdc_mock,heater_mock,pinpuller_mock, PerformanceResult};

const CELCIUS_TO_KELVIN_OFFSET: u16 = 273;

// pub fn emission_sensing<USCI:SerialUsci>(
//     payload: &mut PayloadController<{PayloadOn}, {HeaterOn}>, 
//     spi_bus: &mut PayloadSPIController, 
//     serial: &mut SerialWriter<USCI>){

//     for sensor_result in test_heater(payload, spi_bus, serial).iter(){
//         uwriteln!(serial, "{}", sensor_result).ok();
//     }

//     let fn_arr = [test_cathode_offset, test_tether_bias, test_repeller];
//     for sensor_fn in fn_arr.iter() {
//         for result in sensor_fn(payload, spi_bus, serial).iter() {
//             uwriteln!(serial, "{}", sensor_result).ok();
//         }
//     }
//     uwriteln!(serial, "{}", test_repeller(payload, spi_bus, serial)).ok();
//     print_temperatures(payload, spi_bus, serial);
// }

pub fn deployment_sensing<USCI:SerialUsci>(
    payload: &mut PayloadController<{PayloadOff}, {HeaterOff}>, 
    spi_bus: &mut PayloadSPIController, 
    serial: &mut SerialWriter<USCI>) {
    
    uwriteln!(serial, "{}", test_pinpuller_current_sensor(payload, spi_bus, serial)).ok();
    print_temperatures(payload, spi_bus, serial);
}

pub fn payload_off_sensing<USCI:SerialUsci>(
    payload: &mut PayloadController<{PayloadOff}, {HeaterOff}>, 
    spi_bus: &mut PayloadSPIController, 
    serial: &mut SerialWriter<USCI>) {

    print_temperatures(payload, spi_bus, serial);
}

pub fn print_temperatures<const DONTCARE1:PayloadState, const DONTCARE2:HeaterState, USCI:SerialUsci>(
    payload: &mut PayloadController<{DONTCARE1}, {DONTCARE2}>,
    spi_bus: &mut PayloadSPIController, 
    debug_writer: &mut SerialWriter<USCI>){

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
        let tempr = payload.get_temperature_kelvin(sensor, spi_bus) as i16;
        uwriteln!(debug_writer, "{}: {}", name, tempr - (CELCIUS_TO_KELVIN_OFFSET as i16)).ok();     
    }
    uwriteln!(debug_writer, "").ok();
}

fn test_hvdc_supply<const DONTCARE: HeaterState, USCI:SerialUsci>(
    measure_voltage_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, &mut PayloadSPIController) -> i32,
    measure_current_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, &mut PayloadSPIController) -> i32,
    supply_max: u32,
    payload: &mut PayloadController<{PayloadOn}, DONTCARE>,
    spi_bus: &mut PayloadSPIController,
    debug_writer: &mut SerialWriter<USCI>) -> Fxd {

    dbg_uwriteln!(debug_writer, "");
    
    const SENSE_RESISTANCE: u32 = 1; // Both supplies use the same sense resistor value
        
    // Read voltage, current
    let measured_voltage_mv = measure_voltage_fn(payload, spi_bus);
    let measured_current_ua = measure_current_fn(payload, spi_bus);
    dbg_uwriteln!(debug_writer, "Measured output voltage: {}mV", measured_voltage_mv);
    dbg_uwriteln!(debug_writer, "Measured output current: {}uA", measured_current_ua);

    // Calculate expected voltage and current
    let expected_voltage_mv: i32 = supply_max as i32;

    dbg_uwriteln!(debug_writer, "Expected output voltage: {}mV", expected_voltage_mv);

    let voltage_accuracy = calculate_rpd(measured_voltage_mv, expected_voltage_mv);
    
    voltage_accuracy
}

pub fn test_cathode_offset<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
    payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
    spi_bus: &'a mut PayloadSPIController,
    debug_writer: &mut SerialWriter<USCI>) -> PerformanceResult<'a> {


    let voltage_accuracy = self::test_hvdc_supply(
            &PayloadController::get_cathode_offset_voltage_millivolts, 
            &PayloadController::get_cathode_offset_current_microamps, 
            200_000,
            payload,
            spi_bus, 
            debug_writer);

    let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
    voltage_result
}

pub fn test_tether_bias<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
    payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
    spi_bus: &'a mut PayloadSPIController,
    debug_writer: &mut SerialWriter<USCI>) -> PerformanceResult<'a> {


    let voltage_accuracy = self::test_hvdc_supply(
            &PayloadController::get_tether_bias_voltage_millivolts, 
            &PayloadController::get_tether_bias_current_microamps, 
            200_000,
            payload,
            spi_bus, 
            debug_writer);

    let voltage_result = calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
    voltage_result
}

pub fn test_heater<'a, USCI: SerialUsci>(
    payload: &'a mut PayloadController<{PayloadOn}, {HeaterOn}>, 
    spi_bus: &'a mut PayloadSPIController, 
    debug_writer: &mut SerialWriter<USCI> ) -> [PerformanceResult<'a>; 2] {

    dbg_uwriteln!(debug_writer, "");

    // Read voltage, current
    let heater_voltage_mv = payload.get_heater_voltage_millivolts(spi_bus);
    dbg_uwriteln!(debug_writer, "Read voltage as: {}mV", heater_voltage_mv);
    let heater_current_ma = payload.get_heater_current_milliamps(spi_bus);
    dbg_uwriteln!(debug_writer, "Read current as: {}mA", heater_current_ma);

    // Calculate expected voltage and current
    let expected_voltage_mv: u16 = 2_000;
    let expected_current_ma: i16 = (expected_voltage_mv as u32 * 1000 / heater_mock::CIRCUIT_RESISTANCE_MOHMS as u32)
            .min(heater_mock::POWER_LIMITED_MAX_CURRENT_MA.to_num()) as i16;
    dbg_uwriteln!(debug_writer, "Expected current is: {}mA", expected_current_ma);

    // RPD and accuracy calculations
    let voltage_rpd = calculate_rpd(heater_voltage_mv as i32, expected_voltage_mv as i32);
    let current_rpd = calculate_rpd(heater_current_ma as i32, expected_current_ma as i32);
    

    let voltage_result = calculate_performance_result("Heater voltage", voltage_rpd, 5, 20);
    let current_result = calculate_performance_result("Heater current", current_rpd, 5, 20);

    [voltage_result, current_result]
}

pub fn test_repeller<'a, USCI: SerialUsci, const DONTCARE: HeaterState>(
    payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
    spi_bus: &'a mut PayloadSPIController, 
    debug_writer: &mut SerialWriter<USCI> ) -> PerformanceResult<'a> {
    
    dbg_uwriteln!(debug_writer, "");

    // Read voltage
    let repeller_voltage_mv = payload.get_repeller_voltage_millivolts(spi_bus);
    dbg_uwriteln!(debug_writer, "Read voltage as: {}mV", repeller_voltage_mv);

    // Calculate expected voltage/current
    let expected_voltage_mv: u32 = TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS;

    let voltage_rpd = calculate_rpd(repeller_voltage_mv, expected_voltage_mv as i32);
    

    calculate_performance_result("Repeller voltage", voltage_rpd, 5, 20)
}

pub fn test_pinpuller_current_sensor<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState, USCI:SerialUsci>(
    payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
    spi_bus: &'a mut PayloadSPIController,
    serial_writer: &mut SerialWriter<USCI>) -> PerformanceResult<'a>{

    let measured_current = payload.get_pinpuller_current_milliamps(spi_bus);
    dbg_uwriteln!(serial_writer, "Measured current as {}mA", measured_current);
    let accuracy = calculate_rpd(measured_current as i32, pinpuller_mock::EXPECTED_ON_CURRENT.to_num());

    calculate_performance_result("Pinpuller current sense",  accuracy,  5, 20)
}

pub fn test_board_aperture_current_sensor<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState, USCI:SerialUsci>(
    payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
    spi_bus: &'a mut PayloadSPIController,
    serial_reader: &'a mut Rx<USCI>, 
    serial_writer: &mut SerialWriter<USCI>){


        loop{
            let measured_aperture_adc_mv = payload.aperture_test_adc.read_voltage_from(&APERTURE_TEST_CURRENT_SENSOR, spi_bus);
            let measured_aperture_current_ua = aperture_current_sensor_eq(measured_aperture_adc_mv);
            uwriteln!(serial_writer, "Measured aperture ADC voltage: {}mV", measured_aperture_adc_mv).ok();
            uwriteln!(serial_writer, "Measured aperture current: {}uA", measured_aperture_current_ua).ok(); 

            let tempr = payload.get_temperature_kelvin(&MSP430_TEMPERATURE_SENSOR , spi_bus);
            uwriteln!(serial_writer, "Temperature: {} Kelvin", tempr).ok(); 
            delay_cycles(1_000_000);
        }
        

        // for actual_aperture_current_ua in [100, 200, 400, 600, 800, 1000, 2000, 4000, 6000, 8000]{
        //     uwriteln!(serial_writer, "Set actual current to: {}uA", actual_aperture_current_ua).ok();
        //     uwriteln!(serial_writer, "Press any key to continue").ok();
        //     let dont_care = read_num(serial_writer, serial_reader);
        //     delay_cycles(1_000_000); 

        //     let measured_aperture_adc_mv = payload.aperture_test_adc.read_voltage_from(&APERTURE_TEST_CURRENT_SENSOR, spi_bus);
        //     let measured_aperture_current_ua = aperture_current_sensor_eq(measured_aperture_adc_mv);     
                                      
        //     uwriteln!(serial_writer, "Set aperture current: {}uA", actual_aperture_current_ua).ok();                          
        //     uwriteln!(serial_writer, "Measured aperture ADC voltage: {}mV", measured_aperture_adc_mv).ok();
        //     uwriteln!(serial_writer, "Measured aperture current: {}uA", measured_aperture_current_ua).ok();
        //     uwriteln!(serial_writer, "").ok();
        //     delay_cycles(1_000_000);
        // }
}