 // this file contains PCB-specific structures that are unlikely to change under new versions.

use embedded_hal::digital::v2::OutputPin;

// Structures that group commonly used pins together
use crate::pcb_mapping::pin_name_types::*;
use crate::{Digipot, adc::*, dac::DAC};
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
    pub aperture_test_adc: TetherLMSReceiverEnablePin //For TVAC, used as CS pin for Aperture Test PCB
}
impl PayloadSPIChipSelectPins {
    pub fn new(mut digipot: DigipotCSPin, mut dac: DACCSPin, mut tether_adc: TetherADCCSPin, mut temperature_adc: TemperatureADCCSPin, mut misc_adc: MiscADCCSPin, mut aperture_test_adc: TetherLMSReceiverEnablePin) -> PayloadSPIChipSelectPins{
        digipot.set_high().ok(); // in lieu of accepting stateful output pins just set them high in the constructor
        dac.set_high().ok();
        tether_adc.set_high().ok();
        temperature_adc.set_high().ok();
        misc_adc.set_high().ok();
        aperture_test_adc.set_high().ok();

        PayloadSPIChipSelectPins{digipot, dac, tether_adc, temperature_adc, misc_adc, aperture_test_adc}
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
    //pub lms_receiver_enable: TetherLMSReceiverEnablePin,
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
    pub aperture_test_adc: ApertureTestADC,
}