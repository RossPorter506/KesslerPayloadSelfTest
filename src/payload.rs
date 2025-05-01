// This file provides a high-level interface for interacting with the various sensors on the Kessler payload board.

use core::marker::PhantomData;

use embedded_hal::digital::v2::OutputPin;

use crate::digipot::Digipot; 
use crate::adc::{ApertureADC, MiscADC, TemperatureADC, TemperatureSensor, TetherADC, VccType};
use crate::dac::{DAC, DACCommand};
use crate::spi::{PayloadSPI, PayloadSPIController, SckPolarity::IdleLow, SckPolarity::IdleHigh, SckPhase::SampleFirstEdge};
use crate::pcb_mapping::{sensor_equations::*, sensor_locations::*, power_supply_locations::*, power_supply_limits::*, power_supply_equations::*, PayloadControlPins, PayloadPeripherals};

// Returns num such that "lower bound <= num <= upper_bound"
pub fn enforce_bounds<T: Ord>(lower_bound: T, num: T, upper_bound: T) -> T{
    num.min(upper_bound).max(lower_bound)
}

// Typestates to indicate whether the payload is powered. If the payload is not powered, trying to enable the heater (or really setting any pins connected to the payload) is potentially damaging. 
// Hence valid states are: (PayloadOff, HeaterOff) <-> (PayloadOn, HeaterOff) <-> (PayloadOn, HeaterOn)
#[derive(PartialEq, Eq, core::marker::ConstParamTy)]
pub enum PayloadState {
    PayloadOn,
    PayloadOff,
} use PayloadState::*;

#[derive(PartialEq, Eq, core::marker::ConstParamTy)]
pub enum HeaterState {
    HeaterOn,
    HeaterOff,
} use HeaterState::*;

pub struct PayloadBuilder {}
impl PayloadBuilder{
    pub fn build(periph: PayloadPeripherals, mut pins: PayloadControlPins, spi: PayloadSPIController) -> Payload<{PayloadOff}, {HeaterOff}> {
        pins.heater_enable.set_low().ok();
        pins.payload_enable.set_low().ok();
        
        Payload::<{PayloadOff}, {HeaterOff}>{
            tether_adc: periph.tether_adc, 
            temperature_adc: periph.temperature_adc, 
            misc_adc: periph.misc_adc, 
            aperture_adc: periph.aperture_adc,
            dac: periph.dac, 
            digipot: periph.digipot, 
            pins, spi}
    }
}

pub struct Payload<const PSTATE: PayloadState, const HSTATE: HeaterState> {
    pub tether_adc: TetherADC,
    pub temperature_adc: TemperatureADC,
    pub misc_adc: MiscADC,
    pub aperture_adc: ApertureADC,
    pub dac: DAC,
    pub digipot: Digipot,
    pub spi: PayloadSPIController,
    pins: PayloadControlPins,
}
impl<const PSTATE: PayloadState, const HSTATE: HeaterState> Payload<PSTATE, HSTATE>{
    pub fn return_peripherals(self) -> (PayloadPeripherals, PayloadControlPins){
        (PayloadPeripherals {tether_adc: self.tether_adc, temperature_adc: self.temperature_adc, misc_adc: self.misc_adc, aperture_adc: self.aperture_adc, dac: self.dac, digipot: self.digipot}, self.pins)
    }
}

// Transition functions
impl Payload<{PayloadOff}, {HeaterOff}>{
    pub fn into_enabled_payload(mut self) -> Payload<{PayloadOn}, {HeaterOff}> {
        self.pins.payload_enable.set_high().ok();
        self.dac.send_command(crate::dac::DACCommand::SelectExternalReference, crate::dac::DACChannel::ChannelA, 0x000, self.spi.borrow());
        Payload { tether_adc: self.tether_adc, temperature_adc: self.temperature_adc, misc_adc: self.misc_adc, aperture_adc: self.aperture_adc, dac: self.dac, digipot: self.digipot, 
                            pins: self.pins, spi: self.spi}
    }
}
impl Payload<{PayloadOn}, {HeaterOff}>{
    pub fn into_enabled_heater(mut self) -> Payload<{PayloadOn}, {HeaterOn}> {
        self.pins.heater_enable.set_high().ok();
        Payload { tether_adc: self.tether_adc, temperature_adc: self.temperature_adc, misc_adc: self.misc_adc, aperture_adc: self.aperture_adc, dac: self.dac, digipot: self.digipot, 
                            pins: self.pins, spi: self.spi}
    }
    pub fn into_disabled_payload(mut self) -> Payload<{PayloadOff}, {HeaterOff}> {
        self.pins.payload_enable.set_low().ok();
        Payload { tether_adc: self.tether_adc, temperature_adc: self.temperature_adc, misc_adc: self.misc_adc, aperture_adc: self.aperture_adc, dac: self.dac, digipot: self.digipot, 
                            pins: self.pins, spi: self.spi}
    }
}
impl Payload<{PayloadOn}, {HeaterOn}>{
    pub fn into_disabled_heater(mut self) -> Payload<{PayloadOn}, {HeaterOff}> {
        self.pins.heater_enable.set_low().ok();
        Payload { tether_adc: self.tether_adc, temperature_adc: self.temperature_adc, misc_adc: self.misc_adc, aperture_adc: self.aperture_adc, dac: self.dac, digipot: self.digipot, 
                            pins: self.pins, spi: self.spi}
    }
}
// Actual sensor functions. These are always available.
impl<const PSTATE: PayloadState, const HSTATE: HeaterState> Payload<PSTATE, HSTATE>{
    // Temperature sensors
    // TODO: Remove these once new temperature funciton has been tested
    /*pub fn get_lms_temperature_kelvin(&mut self, temp_sensor: &TemperatureSensor, spi_bus: &mut impl PayloadSPI<IdleHigh,{SampleFirstEdge}>) -> u16{
        let adc_voltage = self.temperature_adc.read_voltage_from(temp_sensor, spi_bus);
        lms_temperature_eq(adc_voltage)
    }
    pub fn get_payload_temperature_kelvin(&mut self, temp_sensor: &TemperatureSensor, spi_bus: &mut impl PayloadSPI<IdleHigh,{SampleFirstEdge}>) -> u16{
        let adc_voltage = self.temperature_adc.read_voltage_from(temp_sensor, spi_bus);
        payload_temperature_eq(adc_voltage)
    }*/
    pub fn get_temperature_kelvin(&mut self, temp_sensor: &TemperatureSensor) -> u16 {
        let adc_voltage = self.temperature_adc.read_voltage_from(temp_sensor, &mut self.spi);
        match &temp_sensor.vcc {
            VccType::LMS     => lms_temperature_eq(adc_voltage),
            VccType::Payload => payload_temperature_eq(adc_voltage)
        }
    }
    // Aperture
    pub fn get_aperture_current_microamps(&mut self) -> u16 {
        // The aperture CS pin also controls whether the aperture ADC and circuitry are powered.
        // They should be powered for at least 5ms before a value is requested.
        self.aperture_adc.cs_pin.set_low().ok();
        crate::delay_cycles(5_000); // 5ms
        
        let adc_voltage = self.aperture_adc.read_voltage_from(&APERTURE_CURRENT_SENSOR, &mut self.spi);
        aperture_current_sensor_eq(adc_voltage)
    }

    // Pinpuller
    pub fn get_pinpuller_current_milliamps(&mut self) -> u16 {
        let adc_voltage = self.misc_adc.read_voltage_from(&PINPULLER_CURRENT_SENSOR, &mut self.spi);
        pinpuller_current_sensor_eq(adc_voltage)
    }

    // LMS
    pub fn get_lms_receiver_1_millivolts(&mut self) -> u16 {
        self.misc_adc.read_voltage_from(&LMS_RECEIVER_1_SENSOR, &mut self.spi)
    }
    pub fn get_lms_receiver_2_millivolts(&mut self) -> u16 {
        self.misc_adc.read_voltage_from(&LMS_RECEIVER_2_SENSOR, &mut self.spi)
    }
    pub fn get_lms_receiver_3_millivolts(&mut self) -> u16 {
        self.misc_adc.read_voltage_from(&LMS_RECEIVER_3_SENSOR, &mut self.spi)
    }
}
// These functions are only available when the payload is on.
impl<const HSTATE: HeaterState> Payload<{PayloadOn}, HSTATE>{
    /* Supplies */
    // Heater
    // Note that we *can* change the heater voltage without the heater being enabled.
    pub fn set_heater_voltage(&mut self, mut target_millivolts: u16){
        target_millivolts = enforce_bounds( 
            HEATER_MIN_VOLTAGE_MILLIVOLTS, 
            target_millivolts, 
            HEATER_MAX_VOLTAGE_MILLIVOLTS);
        let target_digipot_resistance = heater_target_voltage_to_digipot_resistance(target_millivolts);
        self.digipot.set_channel_to_resistance(HEATER_DIGIPOT_CHANNEL,target_digipot_resistance, &mut self.spi);
    }
    pub fn get_heater_voltage_millivolts(&mut self) -> u16{
        let adc_millivolts = self.tether_adc.read_voltage_from(&HEATER_VOLTAGE_SENSOR, &mut self.spi);
        heater_voltage_eq(adc_millivolts)
    }
    pub fn get_heater_current_milliamps(&mut self) -> i16{
        let adc_millivolts = self.tether_adc.read_voltage_from(&HEATER_CURRENT_SENSOR, &mut self.spi);
        heater_current_eq(adc_millivolts)
    }

    // Tether Bias
    pub fn set_tether_bias_voltage(&mut self, mut target_millivolts: u32){
        target_millivolts = enforce_bounds( 
            TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS,
            target_millivolts,
            TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS);
        let dac_voltage = tether_bias_target_voltage_to_dac_voltage(target_millivolts);
        let count = DAC::voltage_to_count(dac_voltage);
        self.dac.send_command(DACCommand::WriteToAndUpdateRegisterX, TETHER_BIAS_SUPPLY_CONTROL_CHANNEL, count, self.spi.borrow())
    }
    pub fn get_tether_bias_voltage_millivolts(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&TETHER_BIAS_VOLTAGE_SENSOR, &mut self.spi);
        tether_bias_voltage_eq(adc_voltage)
    }
    pub fn get_tether_bias_current_microamps(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&TETHER_BIAS_CURRENT_SENSOR, &mut self.spi);
        tether_bias_current_eq(adc_voltage)
    }

    // Cathode Offset
    pub fn set_cathode_offset_voltage(&mut self, mut target_millivolts: u32){
        target_millivolts = enforce_bounds( 
            CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS,
            target_millivolts,
            CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS);
        let dac_voltage = cathode_offset_target_voltage_to_dac_voltage(target_millivolts);
        let count = DAC::voltage_to_count(dac_voltage);
        self.dac.send_command(DACCommand::WriteToAndUpdateRegisterX, CATHODE_OFFSET_SUPPLY_CONTROL_CHANNEL, count, self.spi.borrow())
    }
    pub fn get_cathode_offset_voltage_millivolts(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&CATHODE_OFFSET_VOLTAGE_SENSOR, &mut self.spi);
        cathode_offset_voltage_eq(adc_voltage)
    }
    pub fn get_cathode_offset_current_microamps(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&CATHODE_OFFSET_CURRENT_SENSOR, &mut self.spi);
        cathode_offset_current_eq(adc_voltage)
    }

    // Repeller
    pub fn get_repeller_voltage_millivolts(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&REPELLER_VOLTAGE_SENSOR, &mut self.spi);
        repeller_voltage_eq(adc_voltage)
    }

    // Relays
    pub fn set_cathode_offset_switch(&mut self, state: SwitchState){
        match state{
            SwitchState::Connected => self.pins.cathode_switch.set_high().ok(),
            SwitchState::Disconnected => self.pins.cathode_switch.set_low().ok(),
        };
    }
    pub fn set_tether_bias_switch(&mut self, state: SwitchState){
        match state{
            SwitchState::Connected => self.pins.tether_switch.set_high().ok(),
            SwitchState::Disconnected => self.pins.tether_switch.set_low().ok(),
        };
    }
}

pub enum SwitchState{
    Connected,
    Disconnected,
}