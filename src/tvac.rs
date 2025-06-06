use embedded_hal::{digital::v2::{OutputPin, InputPin}, timer::CountDown};
use msp430fr2355::{E_USCI_A1, TB0};
use msp430fr2x5x_hal::serial::{SerialUsci, Rx};
use msp430fr2x5x_hal::timer::Timer;
use msp430fr2x5x_hal::{pmm::Pmm, gpio::Batch};
use nb::block;
use ufmt::{uWrite, uwrite, uwriteln};
use void::ResultVoidExt;

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

pub fn emission_sensing<USCI:SerialUsci>(
    expected_heater_voltage_mv: u32,
    expected_tb_voltage_mv: u32,
    expected_co_voltage_mv: u32,
    payload: &mut PayloadController<{PayloadOn}, {HeaterOn}>, 
    spi_bus: &mut PayloadSPIController, 
    serial: &mut SerialWriter<USCI>){

    // Compare heater voltage AND current against expected values
    for sensor_result in compare_heater(expected_heater_voltage_mv, payload, spi_bus, serial).iter(){
        uwriteln!(serial, "{}", sensor_result).ok();
    }

    // Compare tether bias and cathode offset voltages against expected values
    let fn_arr          = [compare_cathode_offset, compare_tether_bias];
    let expected_values = [expected_co_voltage_mv, expected_tb_voltage_mv];
    for (sensor_fn, expected_voltage) in fn_arr.iter().zip(expected_values) {
        let result = sensor_fn(expected_voltage, payload, spi_bus, serial);
        uwriteln!(serial, "{}", result).ok();
    }

    // We don't have a good idea of what these *should* be, so just print out their value
    measure_aperture_current(payload, spi_bus, serial);
    measure_repeller_voltage(payload, spi_bus, serial);

    print_temperatures(payload, spi_bus, serial);
}

pub fn deployment_sensing<USCI:SerialUsci>(
    payload: &mut PayloadController<{PayloadOff}, {HeaterOff}>, 
    spi_bus: &mut PayloadSPIController, 
    serial: &mut SerialWriter<USCI>) {
    
    uwriteln!(serial, "{}", compare_pinpuller_current(payload, spi_bus, serial)).ok();
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

fn compare_hvdc_supply<const DONTCARE: HeaterState, USCI:SerialUsci>(
    measure_voltage_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, &mut PayloadSPIController) -> i32,
    measure_current_fn: &dyn Fn(&mut PayloadController<{PayloadOn}, DONTCARE>, &mut PayloadSPIController) -> i32,
    expected_voltage_mv: u32,
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
    let expected_voltage_mv: i32 = expected_voltage_mv as i32;

    dbg_uwriteln!(debug_writer, "Expected output voltage: {}mV", expected_voltage_mv);

    let voltage_accuracy = calculate_rpd(measured_voltage_mv, expected_voltage_mv);
    
    voltage_accuracy
}

pub fn compare_cathode_offset<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
    expected_voltage_mv: u32,
    payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
    spi_bus: &'a mut PayloadSPIController,
    debug_writer: &mut SerialWriter<USCI>) -> PerformanceResult<'a> {

    let voltage_accuracy = self::compare_hvdc_supply(
            &PayloadController::get_cathode_offset_voltage_millivolts, 
            &PayloadController::get_cathode_offset_current_microamps, 
            expected_voltage_mv,
            payload,
            spi_bus, 
            debug_writer);

    let voltage_result = calculate_performance_result("Cathode offset voltage", voltage_accuracy, 5, 20);
    voltage_result
}

pub fn compare_tether_bias<'a, const DONTCARE: HeaterState, USCI:SerialUsci>(
    expected_voltage_mv: u32,
    payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
    spi_bus: &'a mut PayloadSPIController,
    debug_writer: &mut SerialWriter<USCI>) -> PerformanceResult<'a> {

    let voltage_accuracy = self::compare_hvdc_supply(
            &PayloadController::get_tether_bias_voltage_millivolts, 
            &PayloadController::get_tether_bias_current_microamps, 
            expected_voltage_mv,
            payload,
            spi_bus, 
            debug_writer);

    let voltage_result = calculate_performance_result("Tether bias voltage", voltage_accuracy, 5, 20);
    voltage_result
}

pub fn compare_heater<'a, USCI: SerialUsci>(
    expected_voltage_mv: u32,
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
    let expected_voltage_mv = expected_voltage_mv as u16;
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

pub fn measure_repeller_voltage<'a, USCI: SerialUsci, const DONTCARE: HeaterState>(
    payload: &'a mut PayloadController<{PayloadOn}, DONTCARE>, 
    spi_bus: &'a mut PayloadSPIController, 
    debug_writer: &mut SerialWriter<USCI> ) {
    
    dbg_uwriteln!(debug_writer, "");

    // Read voltage
    let repeller_voltage_mv = payload.get_repeller_voltage_millivolts(spi_bus);
    dbg_uwriteln!(debug_writer, "Read voltage as: {}mV", repeller_voltage_mv);

    // Calculate expected voltage/current
    // Do we actually know what the repeller voltage should be?
    //let voltage_rpd = calculate_rpd(repeller_voltage_mv, expected_voltage_mv as i32);
    
    //calculate_performance_result("Repeller voltage", voltage_rpd, 5, 20)
}

pub fn compare_pinpuller_current<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState, USCI:SerialUsci>(
    payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
    spi_bus: &'a mut PayloadSPIController,
    serial_writer: &mut SerialWriter<USCI>) -> PerformanceResult<'a>{

    let measured_current = payload.get_pinpuller_current_milliamps(spi_bus);
    dbg_uwriteln!(serial_writer, "Measured current as {}mA", measured_current);
    let accuracy = calculate_rpd(measured_current as i32, pinpuller_mock::EXPECTED_ON_CURRENT.to_num());

    calculate_performance_result("Pinpuller current sense",  accuracy,  5, 20)
}

pub fn measure_aperture_current<'a, const DONTCARE1: PayloadState, const DONTCARE2:HeaterState, USCI:SerialUsci>(
    payload: &'a mut PayloadController<DONTCARE1, DONTCARE2>, 
    spi_bus: &'a mut PayloadSPIController,
    serial_writer: &mut SerialWriter<USCI>) {

    let measured_current = payload.get_aperture_current_microamps(spi_bus);
    dbg_uwriteln!(serial_writer, "Measured current as {}uA", measured_current);
}

pub fn aperture_current_sense_validation(mut serial_writer: SerialWriter<E_USCI_A1>, payload: &mut PayloadController<{PayloadOn}, {HeaterOn}>, mut payload_spi_controller: PayloadSPIController) {
    // Name of test
    uwriteln!(serial_writer, "========== VACUUM CHAMBER - APERTURE CURRENT SENSE VALIDATION FIRMWARE ==========").ok();
    uwriteln!(serial_writer, "").ok();
    delay_cycles(2_000_000);

    // Automated performance test to ensure setup is correct
    uwriteln!(serial_writer, "========== AUTOMATED PERFORMANCE TEST START ==========").ok();
    uwriteln!(serial_writer, "{}", crate::testing::AutomatedPerformanceTests::test_cathode_offset_voltage(payload, &mut payload_spi_controller, &mut serial_writer)).ok();

    uwriteln!(serial_writer, "========== AUTOMATED PERFORMANCE TEST COMPLETE ==========").ok();
    uwriteln!(serial_writer, "").ok();
    delay_cycles(2_000_000);

    // // Warning to switch off power supply if the test specimen is not in vacuum
    uwriteln!(serial_writer, "========== If the vacuum chamber is not depressurised, please turn off power supply now ==========").ok();
    uwriteln!(serial_writer, "").ok();
    delay_cycles(5_000_000);
    uwriteln!(serial_writer, "========== The vacuum test will initiate in T-: ==========").ok();
    
    for i in 0..20{
        uwriteln!(serial_writer, "========== {} s ==========", 20-i).ok();
        delay_cycles(1_000_000);
    }        

    // Perform electron emission test
    crate::testing::AutomatedPerformanceTests::test_aperture_current_sensor(payload, &mut payload_spi_controller,&mut serial_writer);
    
    uwriteln!(serial_writer, "========== TEST COMPLETE ==========").ok();
}


pub fn tvac_test(payload: PayloadController<{PayloadOff}, {HeaterOff}>, serial_writer: &mut SerialWriter<E_USCI_A1>, mut pinpuller_pins: PinpullerActivationPins, 
    mut payload_spi_controller: PayloadSPIController, mut led_pins: LEDPins, mut timer: Timer<TB0>, mut lms_control_pins: TetherLMSPins) -> ! {
    uwriteln!(serial_writer, "==========TVAC TEST FIRMWARE==========").ok();
    delay_cycles(2_000_000);

    let mut payload = payload.into_enabled_payload(&mut payload_spi_controller).into_enabled_heater();
    crate::testing::AutomatedPerformanceTests::full_system_test(&mut payload, &mut pinpuller_pins, &mut payload_spi_controller, serial_writer);
    let payload = payload.into_disabled_heater().into_disabled_payload();

    led_pins.green_led.set_high().ok();

    timer.start(32768u16);
    let mut sec_elapsed_phase:u32 = 0;
    let mut sec_elapsed_total:u32 = 0;

    //  To avoid 'use of moved value' as we mutate the type of payload between loop iterations(?), we make a second variable to store payload here between loops. 
    let mut payload_off: Option<PayloadController<{PayloadOff}, {HeaterOff}>> = Some(payload);

    loop{
        // ------------------------------------------------------------------------
        // -------------------------- Payload Off ---------------------------------
        // ------------------------------------------------------------------------
        uwriteln!(serial_writer, "ENTERING PAYLOAD-OFF PHASE").ok();
        led_pins.yellow_led.set_low().ok();
        led_pins.red_led.set_low().ok();

        for _ in 0..45*60{
            // LEAVE PAYLOAD OFF FOR 45 MINUTES
            block!(timer.wait()).void_unwrap();
            sec_elapsed_phase += 1;
            sec_elapsed_total += 1;
            uwriteln!(serial_writer, "{} seconds elapsed in the current phase", sec_elapsed_phase).ok();
            uwriteln!(serial_writer, "{} seconds elapsed in the total test", sec_elapsed_total).ok();
            if let Some(payload) = payload_off.as_mut() {
                payload_off_sensing(payload, &mut payload_spi_controller, serial_writer);
            }
            
        }

        uwriteln!(serial_writer, "").ok();
        sec_elapsed_phase = 0;

        // ------------------------------------------------------------------------
        // ----------------------  Pinpuller activation ---------------------------
        // ------------------------------------------------------------------------
        uwriteln!(serial_writer, "ENTERING PINPULLER ACTIVATION PHASE").ok();
        // activate pinpuller and LMS
        pinpuller_pins.burn_wire_1.set_high().ok();
        lms_control_pins.lms_led_enable.set_high().ok();
        lms_control_pins.lms_receiver_enable.set_high().ok();
        led_pins.yellow_led.set_high().ok();

        for _ in 0..60{           
            // LEAVE PINPULLER ON FOR 60 SECONDS
            block!(timer.wait()).void_unwrap();
            sec_elapsed_phase += 1;
            sec_elapsed_total += 1;
            uwriteln!(serial_writer, "{} seconds elapsed in the current phase", sec_elapsed_phase).ok();
            uwriteln!(serial_writer, "{} seconds elapsed in the total test", sec_elapsed_total).ok();
            if let Some(payload) = payload_off.as_mut() {
                deployment_sensing(payload,&mut payload_spi_controller, serial_writer);
            }
            
        }

        // disable pinpuller and LMS
        pinpuller_pins.burn_wire_1.set_low().ok();
        lms_control_pins.lms_led_enable.set_low().ok();
        lms_control_pins.lms_receiver_enable.set_low().ok();

        uwriteln!(serial_writer, "").ok();
        sec_elapsed_phase = 0;

        // ------------------------------------------------------------------------
        // ---------------------------  Emission  ---------------------------------
        // ------------------------------------------------------------------------
        uwriteln!(serial_writer, "ENTERING EMISSION PHASE").ok();
        // Payload On activated for 44 minutes
        let mut payload = payload_off.unwrap().into_enabled_payload(&mut payload_spi_controller).into_enabled_heater();
        
        payload.set_cathode_offset_switch(SwitchState::Connected);
        payload.set_tether_bias_switch(SwitchState::Connected);
        payload.set_cathode_offset_voltage(CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS, &mut payload_spi_controller);
        payload.set_tether_bias_voltage(TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS, &mut payload_spi_controller);
        payload.set_heater_voltage(3160, &mut payload_spi_controller);
        led_pins.red_led.set_high().ok();

        for _ in 0..44*60{
            // ENTER CODE TO READ SENSORS FOR 44 MINUTES
            block!(timer.wait()).void_unwrap();
            sec_elapsed_phase += 1;
            sec_elapsed_total += 1;
            uwriteln!(serial_writer, "{} seconds elapsed in the current phase", sec_elapsed_phase).ok();
            uwriteln!(serial_writer, "{} seconds elapsed in the total test", sec_elapsed_total).ok();
            emission_sensing(3160, TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS, CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS, 
                &mut payload, &mut payload_spi_controller, serial_writer)
        }

        payload.set_cathode_offset_switch(SwitchState::Disconnected);
        payload.set_tether_bias_switch(SwitchState::Disconnected);
        payload_off = Some(payload.into_disabled_heater().into_disabled_payload());

        uwriteln!(serial_writer, "").ok();
        sec_elapsed_phase = 0;
    }
}