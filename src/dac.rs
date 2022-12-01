use embedded_hal::digital::v2::OutputPin;

use crate::{spi::PeripheralSPI, pcb_mapping_v5::PeripheralSPIChipSelectPins};
use crate::dac::{DACCommand::*, DACChannel::*};
const dac_resolution: u16 = 4095;

enum DACCommand{
    WriteToRegisterX=0b000,
	UpdateRegisterX=0b0001,
	WriteToRegisterXAndUpdateAll=0b0010,
	WriteToAndUpdateRegisterX=0b0011,
	PowerOffChannelX=0b100,
	PowerOffChip=0b0101,
	SelectInternalReference=0b0110,
	SelectExternalReference=0b0111,
	NoOp=0b1111,
}

enum DACChannel{
    ChannelA=0b0000,
	ChannelB=0b0001,
	ChannelC=0b0010,
	ChannelD=0b0011,
	AllChannels=0b1111,
}

struct DAC {
    
}
impl DAC{
    fn send_command(&self, spi_bus: &mut dyn PeripheralSPI, cs_pins: &mut PeripheralSPIChipSelectPins,
                    command: DACCommand, channel: DACChannel, value: u16) {
        spi_bus.set_sck_idle_low();
        cs_pins.dac.set_low();
        let payload: u32 = ((command as u32) << 20) | ((channel as u32) << 16) | ((value as u32) << 4);
        spi_bus.send(24, payload);
        cs_pins.dac.set_high();
    }
    fn init(&self, spi_bus: &mut dyn PeripheralSPI, cs_pins: &mut PeripheralSPIChipSelectPins,){
        self. send_command(spi_bus, cs_pins,
                     SelectExternalReference, ChannelA, 0x000);
    }
    fn voltage_to_count(target_voltage_millivolts: u16) -> u16{
        todo!()
    }
}
/*
uint16_t DAC::voltageToCount(uint16_t targetVoltageMillivolts){
	if (targetVoltageMillivolts > DAC_VCC_VOLTAGE_MILLIVOLTS){
		targetVoltageMillivolts = DAC_VCC_VOLTAGE_MILLIVOLTS;
	}
	return ((uint32_t)targetVoltageMillivolts * dacResolution) / DAC_VCC_VOLTAGE_MILLIVOLTS;
}*/