// This file interacts with an LTC2634 Digital to Analog Converter (DAC). 
// PCB-specific values (e.g. reference voltages, channel connections) can be found in the pcb_mapping file.

use embedded_hal::digital::v2::OutputPin;

use crate::pcb_mapping_v5::{DAC_VCC_VOLTAGE_MILLIVOLTS, DacCsPin};
use crate::spi::{PayloadSPI, IdleLow, SampleRisingEdge};
use crate::dac::{DACCommand::*, DACChannel::*};

const DAC_RESOLUTION: u16 = 4095;
pub enum DACCommand{
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

pub enum DACChannel{
    ChannelA=0b0000,
	ChannelB=0b0001,
	ChannelC=0b0010,
	ChannelD=0b0011,
	AllChannels=0b1111,
}

pub struct DAC {
    pub cs_pin: DacCsPin,
}
impl DAC{
    pub fn new(cs_pin: DacCsPin, spi_bus: &mut impl PayloadSPI<IdleLow,SampleRisingEdge>) -> DAC {
        let mut dac = DAC{cs_pin};
        dac.init(spi_bus);
        dac
    }
    pub fn send_command(&mut self, command: DACCommand, channel: DACChannel, value: u16, 
                        spi_bus: &mut impl PayloadSPI<IdleLow, SampleRisingEdge>) {
        self.cs_pin.set_low().ok();
        let payload: u32 = ((command as u32) << 20) | ((channel as u32) << 16) | ((value as u32) << 4);
        spi_bus.send(24, payload);
        self.cs_pin.set_high().ok();
    }
    fn init(&mut self, spi_bus: &mut impl PayloadSPI<IdleLow, SampleRisingEdge>){
        self.send_command(SelectExternalReference, ChannelA, 0x000, spi_bus);
    }
    pub fn voltage_to_count(&self, mut target_millivolts: u16) -> u16{
        if target_millivolts > DAC_VCC_VOLTAGE_MILLIVOLTS {
            target_millivolts = DAC_VCC_VOLTAGE_MILLIVOLTS;
        }
        ((target_millivolts as u32 * DAC_RESOLUTION as u32) / DAC_VCC_VOLTAGE_MILLIVOLTS as u32) as u16
    }
}