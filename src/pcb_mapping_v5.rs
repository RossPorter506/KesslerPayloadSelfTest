// This file acts as an abstraction layer for PCB-specific values that may change between revisions.

pub mod pin_name_types {
    use msp430fr2x5x_hal::gpio::*;

    pub type RedLEDPin = Pin<P2, Pin1, Output>;
    pub type YellowLEDPin = Pin<P2, Pin2, Output>;
    pub type GreenLEDPin = Pin<P2, Pin3, Output>;

    pub type DigipotCSPin =Pin<P6, Pin4, Output>;
    pub type DACCSPin = Pin<P6, Pin3, Output>;
    pub type TetherADCCSPin = Pin<P6, Pin2, Output>;
    pub type TemperatureADCCSPin = Pin<P6, Pin0, Output>;
    pub type MiscADCCSPin = Pin<P5, Pin4, Output>;

    pub type PayloadMISOPin = Pin<P4, Pin7, Alternate1<Output>>; // direction is set up for using the onboard USART peripheral
    pub type PayloadMOSIPin = Pin<P4, Pin6, Alternate1<Output>>;
    pub type PayloadSCKPin =  Pin<P4, Pin5, Alternate1<Output>>;

    pub type PayloadMISOBitBangPin = Pin<P4, Pin7, Input<Pulldown>>; // bitbang version
    pub type PayloadMOSIBitBangPin = Pin<P4, Pin6, Output>;
    pub type PayloadSCKBitBangPin =  Pin<P4, Pin5, Output>;

    pub type OBCMISOPin = Pin<P4, Pin2, Alternate1<Output>>;
    pub type OBCMOSIPin = Pin<P4, Pin3, Alternate1<Output>>;
    pub type OBCSCKPin = Pin<P4, Pin1, Alternate1<Output>>;
    pub type OBCCSPin = Pin<P4, Pin0, Alternate1<Output>>;
    pub type OBCCSInterruptPin = Pin<P2, Pin0, Input<Pullup>>;

    pub type DebugSerialRx = Pin<P4, Pin2, Alternate1<Output>>;
    pub type DebugSerialTx = Pin<P4, Pin3, Alternate1<Output>>;

    pub type PayloadEnablePin = Pin<P6, Pin6, Output>;
    pub type HeaterEnablePin = Pin<P4, Pin4, Output>;
    pub type CathodeSwitchPin = Pin<P3, Pin0, Output>;
    pub type TetherSwitchPin = Pin<P6, Pin1, Output>;

    pub type EndmassSense1Pin = Pin<P5, Pin2, Input<Pulldown>>;
    pub type EndmassSense2Pin = Pin<P3, Pin1, Input<Pulldown>>;
    pub type PinpullerDeploySensePin = Pin<P5, Pin3, Input<Pullup>>;

    pub type BurnWire1Pin = Pin<P3, Pin2, Output>;
    pub type BurnWire1BackupPin = Pin<P3, Pin3, Output>;
    pub type BurnWire2Pin = Pin<P5, Pin0, Output>;
    pub type BurnWire2BackupPin = Pin<P5, Pin1, Output>;

    pub type TetherLMSReceiverEnablePin = Pin<P3, Pin4, Output>;
    pub type TetherLMSLEDEnablePin = Pin<P3, Pin5, Output>;
}
use embedded_hal::digital::v2::OutputPin;
use pin_name_types::*;

use crate::{adc::{MiscADC, TetherADC, TemperatureADC}, dac::DAC, digipot::Digipot};

// Structures that group commonly used pins together
pub struct LEDPins{
    pub red_led: RedLEDPin,
    pub yellow_led: YellowLEDPin,
    pub green_led: GreenLEDPin,
}

pub struct PayloadSPIChipSelectPins{
    pub digipot:        DigipotCSPin, // used to control the heater supply
    pub dac:            DACCSPin, // DAC outputs are used to control the cathode offset and tether bias supply's target voltages
    pub tether_adc:     TetherADCCSPin, //ADC1, measures voltages and currents from tether circuitry
    pub temperature_adc:TemperatureADCCSPin, //ADC2, measures board temperatures
    pub misc_adc:       MiscADCCSPin, //ADC0, measures everything else
}
impl PayloadSPIChipSelectPins {
    pub fn disable_all(&mut self) {
        self.digipot.set_high().ok();
        self.dac.set_high().ok();
        self.tether_adc.set_high().ok();
        self.temperature_adc.set_high().ok();
        self.misc_adc.set_high().ok();
    }
}

//eUSCI_B1
pub struct PayloadSPIPins{
    pub miso: PayloadMISOPin, 
    pub mosi: PayloadMOSIPin, 
    pub sck:  PayloadSCKPin, 
}
pub struct PayloadSPIBitBangPins{
    pub miso: PayloadMISOBitBangPin, 
    pub mosi: PayloadMOSIBitBangPin, 
    pub sck:  PayloadSCKBitBangPin, 
}

//eUSCI_A1
pub struct OBCSPIPins{
    pub miso:                   OBCMISOPin,
    pub mosi:                   OBCMOSIPin,
    pub sck:                    OBCSCKPin,
    pub chip_select:            OBCCSPin,
    pub chip_select_interrupt:  OBCCSInterruptPin, 
}
pub struct DebugSerialPins {
    pub rx: DebugSerialRx,
    pub tx: DebugSerialTx,
}
pub struct PayloadControlPins{
    pub payload_enable: PayloadEnablePin, // turns on most payload devices (power supplies, isolators, etc.)
    pub heater_enable:  HeaterEnablePin, // turns on heater step-down converter
    pub cathode_switch: CathodeSwitchPin, // connects cathode offset+ to exterior
    pub tether_switch:  TetherSwitchPin, // connects tether bias+ to tether
}

pub struct DeploySensePins{
    pub endmass_sense_1:    EndmassSense1Pin, // Detects whether the endmass has ejected
    pub endmass_sense_2:    EndmassSense2Pin, // Detects whether the endmass has ejected
    pub pinpuller_sense:    PinpullerDeploySensePin, // Detects whether the pinpuller has deployed
}

pub struct TetherLMSPins{
    pub lms_receiver_enable: TetherLMSReceiverEnablePin,
    pub lms_led_enable:      TetherLMSLEDEnablePin,
}

pub struct PinpullerActivationPins{
    pub burn_wire_1:        BurnWire1Pin,
    pub burn_wire_1_backup: BurnWire1BackupPin,
    pub burn_wire_2:        BurnWire2Pin,
    pub burn_wire_2_backup: BurnWire2BackupPin,
}

pub struct PayloadPeripherals{
    pub digipot:        Digipot,
    pub dac:            DAC,
    pub tether_adc:     TetherADC, 
    pub temperature_adc:TemperatureADC,
    pub misc_adc:       MiscADC,
}

pub mod power_supply_limits {
    // Maximum and minimum values producable by controllable power supplies
    pub const HEATER_MAX_VOLTAGE_MILLIVOLTS: u16 = 12000;
    pub const HEATER_MIN_VOLTAGE_MILLIVOLTS: u16 = 1400;

    pub const CATHODE_OFFSET_MAX_VOLTAGE_MILLIVOLTS: u32 = 255000;
    pub const CATHODE_OFFSET_MIN_VOLTAGE_MILLIVOLTS: u32 = 0;

    pub const TETHER_BIAS_MAX_VOLTAGE_MILLIVOLTS: u32 = 255000;
    pub const TETHER_BIAS_MIN_VOLTAGE_MILLIVOLTS: u32 = 0;
}
pub mod peripheral_vcc_values {
    // VCC Supply voltages
    pub const ADC_VCC_VOLTAGE_MILLIVOLTS: u16 = 5000; // TODO: Verify
    pub const ISOLATED_ADC_VCC_VOLTAGE_MILLIVOLTS: u16 = 5000; // Verify
    pub const DAC_VCC_VOLTAGE_MILLIVOLTS: u16 = 5000; // TODO: Verify
    pub const PINPULLER_VOLTAGE_MILLIVOLTS: u16 = 3300; // TODO verify
}

/********** Sensor mappings **********/
pub mod sensor_locations {
    use crate::{adc::*};
    // Tether ADC
    pub const REPELLER_VOLTAGE_SENSOR:       TetherSensor = TetherSensor{channel: ADCChannel::IN0};
    pub const HEATER_VOLTAGE_SENSOR:         TetherSensor = TetherSensor{channel: ADCChannel::IN1};
    /**********                             Nothing on channel 2                        **********/
    pub const HEATER_CURRENT_SENSOR:         TetherSensor = TetherSensor{channel: ADCChannel::IN3};
    pub const CATHODE_OFFSET_CURRENT_SENSOR: TetherSensor = TetherSensor{channel: ADCChannel::IN4};
    pub const TETHER_BIAS_CURRENT_SENSOR:    TetherSensor = TetherSensor{channel: ADCChannel::IN5};
    pub const TETHER_BIAS_VOLTAGE_SENSOR:    TetherSensor = TetherSensor{channel: ADCChannel::IN6};
    pub const CATHODE_OFFSET_VOLTAGE_SENSOR: TetherSensor = TetherSensor{channel: ADCChannel::IN7};

    //Temperature ADC
    pub const LMS_EMITTER_TEMPERATURE_SENSOR:       TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN0, vcc: VccType::LMS};
    pub const LMS_RECEIVER_TEMPERATURE_SENSOR:      TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN1, vcc: VccType::LMS};
    pub const MSP430_TEMPERATURE_SENSOR:            TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN2, vcc: VccType::Payload};
    pub const HEATER_SUPPLY_TEMPERATURE_SENSOR:     TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN3, vcc: VccType::Payload};
    pub const HVDC_SUPPLIES_TEMPERATURE_SENSOR:     TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN4, vcc: VccType::Payload};
    pub const TETHER_MONITORING_TEMPERATURE_SENSOR: TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN5, vcc: VccType::Payload};
    pub const TETHER_CONNECTOR_TEMPERATURE_SENSOR:  TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN6, vcc: VccType::Payload};
    pub const MSP_3V3_TEMPERATURE_SENSOR:           TemperatureSensor = TemperatureSensor{channel: ADCChannel::IN7, vcc: VccType::Payload};

    // Misc ADC
    pub const PINPULLER_CURRENT_SENSOR: MiscSensor = MiscSensor{channel: ADCChannel::IN0};
    pub const LMS_RECEIVER_1_SENSOR:    MiscSensor = MiscSensor{channel: ADCChannel::IN1};
    pub const LMS_RECEIVER_2_SENSOR:    MiscSensor = MiscSensor{channel: ADCChannel::IN2};
    pub const LMS_RECEIVER_3_SENSOR:    MiscSensor = MiscSensor{channel: ADCChannel::IN3};
    pub const APERTURE_CURRENT_SENSOR:  MiscSensor = MiscSensor{channel: ADCChannel::IN4};
    /**********                    Nothing after channel 4                     **********/
}
pub mod power_supply_locations {
    use crate::{dac::*, digipot::*};
    // DAC
    pub const CATHODE_OFFSET_SUPPLY_CONTROL_CHANNEL: DACChannel = DACChannel::ChannelC;
    pub const TETHER_BIAS_SUPPLY_CONTROL_CHANNEL: DACChannel = DACChannel::ChannelD;

    // Digipot
    pub const HEATER_DIGIPOT_CHANNEL: DigipotChannel = DigipotChannel::Channel1;
}
/* Sensor equations. Takes in the voltage reported at the ADC (in millivolts) and produces the voltage/current being sensed in millivolts/milliamps */

pub mod sensor_equations {
    use fixed::{FixedI64, types::extra::U32};

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
    pub fn tether_bias_current_eq(v_adc_millivolts: u16) -> i32{ // Output in MICROamps
        //((v_adc_millivolts as i32 - 1020)*1015)/19_608 // original output in 100's of MICROamps, i.e. XXX.X mA. 
        ((v_adc_millivolts as i32 - 1020)*50_750) / 9804
    }   
    pub fn cathode_offset_current_eq(v_adc_millivolts: u16) -> i32{ // output in MICROamps
        ((v_adc_millivolts as i32 - 2463)*780)/500
    }

    pub fn aperture_current_sensor_eq(v_adc_millivolts: u16) -> u16 {
        (((-(v_adc_millivolts as i32) + (40_000/9)) * 43) / 10_000) as u16
    }

    pub fn pinpuller_current_sensor_eq(v_adc_millivolts: u16) -> u16 {
        ((v_adc_millivolts as u32 * 1000) / 1804) as u16
    }

    //Returns temperature in Kelvin
    pub fn payload_temperature_eq(v_adc_millivolts: u16) -> u16 {
        generic_temperature_eq(v_adc_millivolts, 5000)
    }
    pub fn lms_temperature_eq(v_adc_millivolts: u16) -> u16 {
        generic_temperature_eq(v_adc_millivolts, 3300)
    }
    fn generic_temperature_eq(v_adc_millivolts: u16, vcc: u16) -> u16 {
        let ln_millivolts_approx = FixedI64::<U32>::from(FixedI64::<U32>::from(v_adc_millivolts).int_log10()) / FixedI64::LOG10_E;
        //let ln_millivolts_approx = (u16::ilog2(v_adc_millivolts) - u16::ilog10(v_adc_millivolts)) as u16; // approximate ln using integer logs
        (FixedI64::<U32>::from(1_028_100) / ( FixedI64::<U32>::from(705)+298*(FixedI64::<U32>::from(v_adc_millivolts))*10_000/(FixedI64::<U32>::from(vcc)-ln_millivolts_approx) )).saturating_to_num()
    }
}
/* Supply control equations */
pub mod power_supply_equations {
    use fixed::{types::extra::U31, FixedI64};

    pub fn heater_target_voltage_to_digipot_resistance(millivolts: u32) -> u32{
        (FixedI64::<U31>::from(75_000) / ((FixedI64::<U31>::from(millivolts))/810 - FixedI64::<U31>::from(1))).saturating_to_num()
    }

    pub fn tether_bias_target_voltage_to_dac_voltage(millivolts: u32) -> u16{
        (millivolts / 51) as u16
    }
    pub fn cathode_offset_target_voltage_to_dac_voltage(millivolts: u32) -> u16{
        ((millivolts * 100) / 5138) as u16
    }
}

pub const TETHER_SENSE_RESISTANCE_OHMS: u32 = 1;
pub const CATHODE_SENSE_RESISTANCE_OHMS: u32 = 1;
pub const HEATER_SENSE_RESISTANCE_MILLIOHMS: u32 = 10;