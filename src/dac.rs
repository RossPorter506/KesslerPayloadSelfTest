// This file interacts with an LTC2634 Digital to Analog Converter (DAC). 
// PCB-specific values (e.g. reference voltages, channel connections) can be found in the pcb_mapping file.

use crate::pcb_mapping::{peripheral_vcc_values::DAC_VCC_VOLTAGE_MILLIVOLTS, pin_name_types::DACCSPin};
use crate::spi::{PayloadSPI, IdleLow, SampleFirstEdge};
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

// Packet format: C3 C2 C1 C0 A3 A2 A1 A0 D11 D10 D9 D8 D7 D6 D5 D4 D3 D2 D1 D0 X X X X
//                24...                                                            ...0
// Where C is command bits, A is address, D is data, and X is 'dont care'

const NUM_COMMAND_BITS: u8 = 4;
const NUM_ADDRESS_BITS: u8 = 4;
const NUM_DATA_BITS: u8 = 12;
const NUM_DONT_CARE_BITS: u8 = 4;
const NUM_BITS_IN_PACKET: u8 = NUM_COMMAND_BITS + NUM_ADDRESS_BITS + NUM_DATA_BITS + NUM_DONT_CARE_BITS;

const DATA_OFFSET: u8 = NUM_DONT_CARE_BITS;
const ADDRESS_OFFSET: u8 = NUM_DONT_CARE_BITS + NUM_DATA_BITS;
const COMMAND_OFFSET: u8 = NUM_ADDRESS_BITS + NUM_DONT_CARE_BITS + NUM_DATA_BITS;

pub struct DAC {
    pub cs_pin: DACCSPin,
}
impl DAC{
    pub fn new(cs_pin: DACCSPin, spi_bus: &mut impl PayloadSPI<IdleLow, SampleFirstEdge>) -> DAC {
        let mut dac = DAC{cs_pin};
        dac.send_command(SelectExternalReference, ChannelA, 0x000, spi_bus);
        dac
    }
    pub fn send_command(&mut self, command: DACCommand, channel: DACChannel, value: u16, 
                        spi_bus: &mut impl PayloadSPI<IdleLow, SampleFirstEdge>) {
        let payload: u32 = ((command as u32) << COMMAND_OFFSET) | ((channel as u32) << ADDRESS_OFFSET) | ((value as u32) << DATA_OFFSET);
        spi_bus.send(NUM_BITS_IN_PACKET, payload, &mut self.cs_pin);
    }
    pub fn voltage_to_count(&self, mut target_millivolts: u16) -> u16{
        if target_millivolts > DAC_VCC_VOLTAGE_MILLIVOLTS {
            target_millivolts = DAC_VCC_VOLTAGE_MILLIVOLTS;
        }
        ((target_millivolts as u32 * DAC_RESOLUTION as u32) / DAC_VCC_VOLTAGE_MILLIVOLTS as u32) as u16
    }
}