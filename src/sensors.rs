// This file provides a high-level interface for interacting with the various sensors on the Kessler payload board.

use crate::{adc::{TemperatureSensor,TetherADC, MiscADC, TemperatureADC}, dac::{DAC, DACCommand}, digipot::Digipot, spi::PayloadSPI}; 
use crate::pcb_mapping_v5::*;

// Returns num such that "lower bound <= num <= upper_bound"
pub fn enforce_bounds<T: PartialOrd>(lower_bound: T, mut num: T, upper_bound: T) -> T{
    if num > upper_bound {
        num = upper_bound;
    }
    else if num < lower_bound {
        num = lower_bound;
    }
    num
}

pub struct PayloadController<'a> {
    pub tether_adc: TetherADC,
    pub temperature_adc: TemperatureADC,
    pub misc_adc: MiscADC,
    pub dac: DAC,
    pub digipot: Digipot,
    pub spi_bus: &'a mut dyn PayloadSPI,
}
impl PayloadController<'_>{
    pub fn new( tether_adc: TetherADC, 
                temperature_adc: TemperatureADC,
                misc_adc: MiscADC,
                dac: DAC,
                digipot: Digipot,
                spi_bus: &mut impl PayloadSPI) -> PayloadController {
        PayloadController{tether_adc, temperature_adc, misc_adc, dac, digipot, spi_bus}
    }
    pub fn return_parts(self) -> (TetherADC, TemperatureADC, MiscADC, DAC, Digipot){
        (self.tether_adc, self.temperature_adc, self.misc_adc, self.dac, self.digipot)
    }
    //Temperature sensors
    pub fn get_lms_temperature_kelvin(&mut self, temp_sensor: &TemperatureSensor) -> u16{
        let adc_voltage = self.temperature_adc.read_voltage_from(temp_sensor, self.spi_bus);
        lms_temperature_eq(adc_voltage)
    }
    pub fn get_payload_temperature_kelvin(&mut self, temp_sensor: &TemperatureSensor) -> u16{
        let adc_voltage = self.temperature_adc.read_voltage_from(temp_sensor, self.spi_bus);
        payload_temperature_eq(adc_voltage)
    }

    /* Supplies */
    // Heater
    pub fn set_heater_voltage(&mut self, mut target_voltage_millivolts: u16){
        target_voltage_millivolts = enforce_bounds( HEATER_MIN_VOLTAGE_MILLIVOLTS, 
                                                    target_voltage_millivolts, 
                                                    HEATER_MAX_VOLTAGE_MILLIVOLTS);
        let target_digipot_resistance = heater_target_voltage_to_digipot_resistance(target_voltage_millivolts as f32);
        self.digipot.set_channel_to_resistance(HEATER_DIGIPOT_CHANNEL,target_digipot_resistance, self.spi_bus);
    }
    pub fn get_heater_voltage_millivolts(&mut self) -> u16{
        let adc_voltage_millivolts = self.tether_adc.read_voltage_from(&HEATER_VOLTAGE_SENSOR, self.spi_bus);
        heater_voltage_eq(adc_voltage_millivolts)
    }
    pub fn get_heater_current_milliamps(&mut self) -> i16{
        let adc_voltage_millivolts = self.tether_adc.read_voltage_from(&HEATER_CURRENT_SENSOR, self.spi_bus);
        heater_current_eq(adc_voltage_millivolts)
    }

    //Tether Bias
    pub fn set_tether_bias_voltage(&mut self, mut target_voltage_millivolts: u32){
        target_voltage_millivolts = enforce_bounds( TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS,
                                                    target_voltage_millivolts,
                                                    TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS);
        let dac_voltage = tether_bias_target_voltage_to_dac_voltage(target_voltage_millivolts);
        let count = self.dac.voltage_to_count(dac_voltage);
        self.dac.send_command(DACCommand::WriteToAndUpdateRegisterX, TETHER_BIAS_SUPPLY_CONTROL_CHANNEL, count, self.spi_bus)
    }
    pub fn get_tether_bias_voltage_millivolts(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&TETHER_BIAS_VOLTAGE_SENSOR, self.spi_bus);
        tether_bias_voltage_eq(adc_voltage)
    }
    pub fn get_tether_bias_current_milliamps(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&TETHER_BIAS_CURRENT_SENSOR, self.spi_bus);
        tether_bias_current_eq(adc_voltage)
    }

    //Cathode Offset
    pub fn set_cathode_offset_voltage(&mut self, mut target_voltage_millivolts: u32){
        target_voltage_millivolts = enforce_bounds( CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS,
                                                    target_voltage_millivolts,
                                                    CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS);
        let dac_voltage = cathode_offset_target_voltage_to_dac_voltage(target_voltage_millivolts);
        let count = self.dac.voltage_to_count(dac_voltage);
        self.dac.send_command(DACCommand::WriteToAndUpdateRegisterX, CATHODE_OFFSET_SUPPLY_CONTROL_CHANNEL, count, self.spi_bus)
    }
    pub fn get_cathode_offset_voltage_millivolts(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&CATHODE_OFFSET_VOLTAGE_SENSOR, self.spi_bus);
        cathode_offset_voltage_eq(adc_voltage)
    }
    pub fn get_cathode_offset_current_milliamps(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&CATHODE_OFFSET_CURRENT_SENSOR, self.spi_bus);
        cathode_offset_current_eq(adc_voltage)
    }

    //Repeller
    pub fn get_repeller_voltage_millivolts(&mut self) -> i32 {
        let adc_voltage = self.tether_adc.read_voltage_from(&REPELLER_VOLTAGE_SENSOR, self.spi_bus);
        repeller_voltage_eq(adc_voltage)
    }
}

