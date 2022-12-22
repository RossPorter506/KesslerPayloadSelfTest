// This file interacts with an AD5162 Digital potentiometer. 
// PCB-specific values (e.g. channel connections) can be found in the pcb_mapping file.


// Digipot parameters
pub const DIGIPOT_MAX_RESISTANCE: u32 = 100000;
pub const DIGIPOT_WIPER_RESISTANCE: u32 = 100;
pub const DIGIPOT_RESOLUTION: u32 = 255;

use crate::{spi::{PayloadSPI, IdleLow, SampleFirstEdge}, pcb_mapping_v5::pin_name_types::DigipotCSPin};
use embedded_hal::digital::v2::OutputPin;
use crate::sensors::enforce_bounds;

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
    pub fn set_channel_to_resistance(&mut self, channel: DigipotChannel, wanted_resistance: u32, spi_bus: &mut impl PayloadSPI<IdleLow, SampleFirstEdge>){
        let count = self.resistance_to_count(wanted_resistance);
        self.set_channel_to_count(channel, count, spi_bus);
    }
    pub fn set_channel_to_count(&mut self, channel: DigipotChannel, count: u8, spi_bus: &mut impl PayloadSPI<IdleLow, SampleFirstEdge>){
        let payload = ((channel as u16) << 8 + count) as u32;
        self.cs_pin.set_low().ok();
        spi_bus.send(16, payload);
        self.cs_pin.set_high().ok();
    }
    pub fn resistance_to_count(&self, mut wanted_resistance: u32) -> u8{
        wanted_resistance = enforce_bounds( DIGIPOT_WIPER_RESISTANCE, 
                                            wanted_resistance,
                                            DIGIPOT_MAX_RESISTANCE);
        (((wanted_resistance - DIGIPOT_WIPER_RESISTANCE) * DIGIPOT_RESOLUTION) / DIGIPOT_MAX_RESISTANCE) as u8
    }
}