use msp430fr2x5x_hal::gpio::{*};

pub struct LEDPins{
    pub red_led: Pin<P2, Pin1, Output>,
    pub yellow_led: Pin<P2, Pin2, Output>,
    pub green_led: Pin<P2, Pin3, Output>,
}

pub struct PeripheralSPIChipSelectPins{
    pub heater_digipot:         Pin<P6, Pin4, Output>, // used to control the heater supply
    pub dac:                    Pin<P6, Pin3, Output>, // DAC outputs are used to control the cathode offset and tether bias supply's target voltages
    pub tether_measurement_adc: Pin<P6, Pin2, Output>, //ADC1, measures voltages and currents from tether circuitry
    pub board_temperature_adc:  Pin<P6, Pin0, Output>, //ADC2, measures board temperatures
    pub misc_adc:               Pin<P5, Pin4, Output>, //ADC0, measures everything else
}

//eUSCI_B1
pub struct PayloadSPIPins{
    pub miso: Pin<P6, Pin4, Alternate1<Output>>, 
    pub mosi: Pin<P6, Pin3, Alternate1<Output>>, 
    pub sck:  Pin<P6, Pin2, Alternate1<Output>>, 
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

