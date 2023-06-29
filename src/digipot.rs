// This file interacts with an AD5162 Digital potentiometer. 
// PCB-specific values (e.g. channel connections) can be found in the pcb_mapping file.


// Digipot parameters
pub const DIGIPOT_MAX_RESISTANCE: u32 = 100_000;
pub const DIGIPOT_WIPER_RESISTANCE: u32 = 100;
pub const DIGIPOT_RESOLUTION: u32 = 255;
const DIGIPOT_NUM_ADDRESS_BITS: u8 = 1;
const DIGIPOT_NUM_DATA_BITS: u8 = 8;
const DIGIPOT_NUM_BITS_IN_PACKET: u8 = DIGIPOT_NUM_ADDRESS_BITS + DIGIPOT_NUM_DATA_BITS;

use crate::{spi::{PayloadSPI, PayloadSPIController, SckPolarity::IdleLow, SckPhase::SampleFirstEdge}, pcb_mapping::pin_name_types::DigipotCSPin};
use crate::payload::enforce_bounds;

pub enum DigipotChannel{
	Channel1=0,
	Channel2=1,
}

pub struct Digipot {
    cs_pin: DigipotCSPin,
}
impl Digipot {
    pub fn new(cs_pin: DigipotCSPin) -> Digipot {
        Digipot {cs_pin}
    }
    pub fn set_channel_to_resistance(&mut self, channel: DigipotChannel, wanted_resistance: u32, spi_bus: &mut PayloadSPIController){
        let count = self.resistance_to_count(wanted_resistance);
        self.set_channel_to_count(channel, count, spi_bus.borrow());
    }
    pub fn set_channel_to_count(&mut self, channel: DigipotChannel, count: u8, spi_bus: &mut impl PayloadSPI<{IdleLow}, {SampleFirstEdge}>){
        let payload: u16 = ((channel as u16) << DIGIPOT_NUM_DATA_BITS) | (count as u16);
        spi_bus.send(DIGIPOT_NUM_BITS_IN_PACKET, payload as u32, &mut self.cs_pin);
    }
    pub fn resistance_to_count(&self, mut wanted_resistance: u32) -> u8{
        wanted_resistance = enforce_bounds( DIGIPOT_WIPER_RESISTANCE, 
                                            wanted_resistance,
                                            DIGIPOT_MAX_RESISTANCE);
        (((wanted_resistance - DIGIPOT_WIPER_RESISTANCE) * DIGIPOT_RESOLUTION) / DIGIPOT_MAX_RESISTANCE) as u8
    }
}