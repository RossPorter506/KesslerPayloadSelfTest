use libm::log;
use msp430fr2x5x_hal::gpio::*;

use crate::{adc::{TetherSensor, ADCChannel, TemperatureSensor, MiscSensor}, dac::DACChannel, digipot::DigipotChannel};

pub struct LEDPins{
    pub red_led: Pin<P2, Pin1, Output>,
    pub yellow_led: Pin<P2, Pin2, Output>,
    pub green_led: Pin<P2, Pin3, Output>,
}

pub struct PayloadSPIChipSelectPins{
    pub heater_digipot:         Pin<P6, Pin4, Output>, // used to control the heater supply
    pub dac:                    Pin<P6, Pin3, Output>, // DAC outputs are used to control the cathode offset and tether bias supply's target voltages
    pub tether_measurement_adc: Pin<P6, Pin2, Output>, //ADC1, measures voltages and currents from tether circuitry
    pub board_temperature_adc:  Pin<P6, Pin0, Output>, //ADC2, measures board temperatures
    pub misc_adc:               Pin<P5, Pin4, Output>, //ADC0, measures everything else
}

//eUSCI_B1
pub struct PayloadSPIPins{
    pub miso: Pin<P4, Pin7, Alternate1<Output>>, 
    pub mosi: Pin<P4, Pin6, Alternate1<Output>>, 
    pub sck:  Pin<P4, Pin5, Alternate1<Output>>, 
}

//eUSCI_A1
pub struct OBCSPIPins{
    pub miso:                   Pin<P4, Pin2, Alternate1<Output>>, //direction is DontCare
    pub mosi:                   Pin<P4, Pin3, Alternate1<Output>>, //direction is DontCare
    pub sck:                    Pin<P4, Pin1, Alternate1<Output>>, //direction is DontCare
    pub chip_select:            Pin<P4, Pin0, Alternate1<Output>>, //direction is DontCare
    pub chip_select_interrupt:  Pin<P2, Pin0, Input<Pullup>>, 
}

pub struct PayloadControlPins{
    pub payload_enable: Pin<P6, Pin6, Output>, // turns on most payload devices (power supplies, isolators, etc.)
    pub heater_enable:  Pin<P4, Pin4, Output>, // turns on heater step-down converter
    pub cathode_switch: Pin<P3, Pin0, Output>, // connects cathode offset+ to exterior
    pub tether_switch:   Pin<P6, Pin1, Output>, // connects tether bias+ to tether
}

pub struct DeploySensePins{
    pub deploy_sense_1:         Pin<P5, Pin2, Input<Pulldown>>, // Detects whether the endmass has ejected
    pub deploy_sense_2:         Pin<P3, Pin1, Input<Pulldown>>, // Detects whether the endmass has ejected
    pub pinpuller_deploy_sense: Pin<P5, Pin3, Input<Pullup>>, // Detects whether the pinpuller has deployed
}

pub struct BurnWires{
    pub burn_wire_1:        Pin<P3, Pin2, Output>, // Primary pinpuller activation
    pub burn_wire_1_backup: Pin<P3, Pin3, Output>, // Backup
    pub burn_wire_2:        Pin<P5, Pin0, Output>, // Auxiliary pinpuller activation
    pub burn_wire_2_backup: Pin<P5, Pin1, Output>, // Backup
}

pub struct TetherLMSPins{
    pub tether_lms_receiver_enable: Pin<P3, Pin4, Output>, // Detects whether the endmass has ejected
    pub tether_lms_led_enable:      Pin<P3, Pin5, Output>, // Detects whether the endmass has ejected
}

// Maximum and minimum values producable by controllable power supplies
pub const HEATER_MAX_VOLTAGE_MILLIVOLTS: u16 = 12000;
pub const HEATER_MIN_VOLTAGE_MILLIVOLTS: u16 = 1400;

pub const CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS: u32 = 255000;
pub const CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS: u32 = 0;

pub const TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS: u32 = 255000;
pub const TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS: u32 = 0;

// VCC Supply voltages
pub const ADC_VCC_VOLTAGE_MILLIVOLTS: u16 = 5000; // TODO: Verify
pub const ISOLATED_ADC_VCC_VOLTAGE_MILLIVOLTS: u16 = 5100; // Verify
pub const DAC_VCC_VOLTAGE_MILLIVOLTS: u16 = 5100; // TODO: Verify

// Digipot parameters
pub const DIGIPOT_MAX_RESISTANCE: u32 = 100000;
pub const DIGIPOT_WIPER_RESISTANCE: u32 = 100;
pub const DIGIPOT_RESOLUTION: u32 = 255;

/********** Sensor mappings **********/
// Tether ADC
pub const REPELLER_VOLTAGE_SENSOR: TetherSensor =       TetherSensor{channel: ADCChannel::IN0};
pub const HEATER_VOLTAGE_SENSOR: TetherSensor =         TetherSensor{channel: ADCChannel::IN1};
/*                                                                  Nothing on channel 2     */
pub const HEATER_CURRENT_SENSOR: TetherSensor =         TetherSensor{channel: ADCChannel::IN3};
pub const CATHODE_OFFSET_CURRENT_SENSOR: TetherSensor = TetherSensor{channel: ADCChannel::IN4};
pub const TETHER_BIAS_CURRENT_SENSOR: TetherSensor =    TetherSensor{channel: ADCChannel::IN5};
pub const TETHER_BIAS_VOLTAGE_SENSOR: TetherSensor =    TetherSensor{channel: ADCChannel::IN6};
pub const CATHODE_OFFSET_VOLTAGE_SENSOR: TetherSensor = TetherSensor{channel: ADCChannel::IN7};

//Temperature ADC
pub const LMS_EMITTER_TEMPERATURE_SENSOR: TemperatureSensor =       TemperatureSensor{channel: ADCChannel::IN0};
pub const LMS_RECEIVER_TEMPERATURE_SENSOR: TemperatureSensor =      TemperatureSensor{channel: ADCChannel::IN1};
pub const MSP430_TEMPERATURE_SENSOR: TemperatureSensor =            TemperatureSensor{channel: ADCChannel::IN2};
pub const HEATER_SUPPLY_TEMPERATURE_SENSOR: TemperatureSensor =     TemperatureSensor{channel: ADCChannel::IN3};
pub const HVDC_SUPPLIES_TEMPERATURE_SENSOR: TemperatureSensor =     TemperatureSensor{channel: ADCChannel::IN4};
pub const TETHER_MONITORING_TEMPERATURE_SENSOR: TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN5};
pub const TETHER_CONNECTOR_TEMPERATURE_SENSOR: TemperatureSensor =  TemperatureSensor{channel: ADCChannel::IN6};
pub const MSP_3V3_TEMPERATURE_SENSOR: TemperatureSensor =           TemperatureSensor{channel: ADCChannel::IN7};

// Misc ADC
pub const PINPULLER_CURRENT_SENSOR: MiscSensor =    MiscSensor{channel: ADCChannel::IN0};
pub const LMS_RECEIVER_1_SENSOR: MiscSensor =       MiscSensor{channel: ADCChannel::IN1};
pub const LMS_RECEIVER_2_SENSOR: MiscSensor =       MiscSensor{channel: ADCChannel::IN2};
pub const LMS_RECEIVER_3_SENSOR: MiscSensor =       MiscSensor{channel: ADCChannel::IN3};
pub const APERTURE_CURRENT_SENSOR: MiscSensor =     MiscSensor{channel: ADCChannel::IN4};

// DAC
pub const CATHODE_OFFSET_SUPPLY_CONTROL_CHANNEL: DACChannel = DACChannel::ChannelC;
pub const TETHER_BIAS_SUPPLY_CONTROL_CHANNEL: DACChannel = DACChannel::ChannelD;

//Digipot
pub const HEATER_DIGIPOT_CHANNEL: DigipotChannel = DigipotChannel::Channel1;

/* Sensor equations. Takes in the voltage reported at the ADC (in millivolts) and produces the voltage/current being sensed in millivolts/milliamps */

pub fn heater_voltage_eq(v_adc_millivolts: u16) -> u16{
    ((v_adc_millivolts as i32 * 1035)/310) as u16
}
pub fn repeller_voltage_eq(v_adc_millivolts: u16) -> i32{
    (v_adc_millivolts as i32 - 2755)*102
}
pub fn tether_bias_voltage_eq(v_adc_millivolts: u16) -> i32{
    (v_adc_millivolts as i32 * 106)+805
}
pub fn cathode_offset_voltage_eq(v_adc_millivolts: u16) -> i32{
    ((v_adc_millivolts as i32 * -86_463)/1000)+301_437
}
pub fn heater_current_eq(v_adc_millivolts: u16) -> i16{
    (((v_adc_millivolts as i32 * 2*957)/1000)-66) as i16
}
pub fn tether_bias_current_eq(v_adc_millivolts: u16) -> i32{
    ((v_adc_millivolts as i32 - 1020)*1015)/19_608
}
pub fn cathode_offset_current_eq(v_adc_millivolts: u16) -> i32{
    ((v_adc_millivolts as i32 - 2463)*780)/500
}

//Returns temperature in Kelvin
pub fn payload_temperature_eq(v_adc_millivolts: u16) -> u16 {
    let log_millivolts = log(v_adc_millivolts as f64) as f32;
    (1_028_100.0 / ( 705.0+298.0*(v_adc_millivolts as f32)*10_000.0/(5000.0-log_millivolts) )) as u16
}
pub fn lms_temperature_eq(v_adc_millivolts: u16) -> u16 {
    let log_millivolts = log(v_adc_millivolts as f64) as f32;
    (1_028_100.0 / ( 705.0+298.0*(v_adc_millivolts as f32)*10_000.0/(3300.0-log_millivolts) )) as u16
}

/* Supply control equations */

pub fn heater_target_voltage_to_digipot_resistance(millivolts: f32) -> u32{
    (75_000.0 / ((millivolts)/810.0 - 1.0)) as u32
}

pub fn tether_bias_target_voltage_to_dac_voltage(millivolts: u32) -> u16{
    (millivolts / 51) as u16
}
pub fn cathode_offset_target_voltage_to_dac_voltage(millivolts: u32) -> u16{
    ((millivolts * 100) / 5138) as u16
}