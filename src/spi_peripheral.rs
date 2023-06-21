use core::convert::TryInto;

/// An abstraction layer (adds typestating) built on top of an embedded-hal implmentation. 
use crate::pcb_mapping::{OBCSPIPins, PayloadSPIPins, pin_name_types::{PayloadMOSIBitBangPin, PayloadMISOBitBangPin, PayloadSCKBitBangPin}, PayloadSPIBitBangPins};
use embedded_hal::{digital::v2::{OutputPin, ToggleableOutputPin, InputPin}, prelude::_embedded_hal_blocking_spi_Transfer};
use msp430fr2355::E_USCI_B1;
use msp430fr2x5x_hal::{gpio::*, spi::{BitOrder, BitCount, SpiBus}};
pub use msp430fr2x5x_hal::spi::{Polarity, Phase, SpiError}; // Re-export from HAL
use nb::block;
use crate::delay_cycles;



// Trait because we can implement by either bitbanging or using peripheral
// Separate traits befause OBC_SPI might be expanded in the future (e.g. pin interrupts)
pub trait OBCSPI{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_receive(&mut self, len: u8, data: u32) -> u32;
}
pub trait PayloadSPI<const POLARITY: Polarity, const PHASE: Phase>{
    /// Send a series of packets.
    fn send(&mut self, data: &mut [u8], cs_pin: &mut impl OutputPin);
    /// Receive a number of packets, sending all-zero packets.
    fn receive(&mut self, buffer: &mut [u8], cs_pin: &mut impl OutputPin);
    /// Send a number of packets while receiving at the same time (duplex).
    fn send_receive(&mut self, data: &mut [u8], cs_pin: &mut impl OutputPin);
}

/// Payload SPI implementation that uses bit banging.
pub struct PayloadSPIPeripheral<const POLARITY: Polarity, const PHASE: Phase>{
    pub bus: SpiBus<E_USCI_B1>  //TODO: Make private once debugging finished
}

impl<const POLARITY: Polarity, const PHASE: Phase> PayloadSPIPeripheral<POLARITY, PHASE>{
    /// Create a new SPI bus by consuming SPI pins.
    pub fn new(bus: SpiBus<E_USCI_B1>) -> Self {
        Self {bus}
    }
    pub fn return_pins(self) -> PayloadSPIPins {
        let (sck, mosi, miso) = self.bus.return_pins();
        PayloadSPIPins{sck, mosi, miso}
    }
    /// Consumes the old bus to produces a new one of a different type. Output type is usually inferred automatically.
    pub fn into<const NEW_POL: Polarity, const NEW_PHA: Phase>(mut self) -> PayloadSPIPeripheral<NEW_POL, NEW_PHA>{
        self.bus.reconfigure(NEW_POL, NEW_PHA, BitOrder::MsbFirst, BitCount::EightBits);
        PayloadSPIPeripheral::<NEW_POL, NEW_PHA>{bus: self.bus}
    }
}

// Actual trait implementations
impl<const POLARITY: Polarity, const PHASE: Phase> PayloadSPI<POLARITY, PHASE> for PayloadSPIPeripheral<POLARITY, PHASE> {
    fn send(&mut self, data: &mut [u8], cs_pin: &mut impl OutputPin) {
        self.send_receive(data, cs_pin);
    }
    fn receive(&mut self, buf: &mut [u8], cs_pin: &mut impl OutputPin) {
        buf.fill(0);
        self.send_receive(buf, cs_pin);
    }
    fn send_receive(&mut self, data: &mut [u8], cs_pin: &mut impl OutputPin) { 
        cs_pin.set_low().ok();
        self.bus.transfer(data).ok(); // we should probably check for overruns here, but eh.
        block!(self.bus.flush()).ok();
        cs_pin.set_high().ok();
     }
}

/// A wrapper class that automates changing the typestate of the bus. Useful for intermediate functions that don't use the bus themselves, but call functions that do.
/// 
/// Functions that require the SPI bus can borrow it using .borrow()
pub struct PayloadSPIController {
    // It looks like we can only store one type here, but we'll convert it in .borrow().
    // Trust me, this is the easiest way.
    pub spi_bus: PayloadSPIPeripheral<{Polarity::IdleHigh}, {Phase::CaptureOnFirstEdge}> //TODO: Make private once debugging finished
}
impl PayloadSPIController {
    /// Generates a new controller by consuming an existing SPI bus.
    pub fn new_from_bus<const POLARITY: Polarity, const PHASE: Phase>(bus: PayloadSPIPeripheral<POLARITY, PHASE>) -> Self {
        Self {spi_bus: bus.into()}
    }
    pub fn new(bus: SpiBus<E_USCI_B1>) -> Self {
        let spi_bus = PayloadSPIPeripheral::new(bus);
        Self {spi_bus}
    }
    pub fn return_bus<const POLARITY: Polarity, const PHASE: Phase>(self) -> PayloadSPIPeripheral<POLARITY, PHASE> {
        self.spi_bus.into()
    }
    pub fn return_pins<const POLARITY: Polarity, const PHASE: Phase>(self) -> PayloadSPIPins {
        self.spi_bus.return_pins()
    }
    /// Return a mutable reference to the SPI bus, converting to the correct typestate as required.
    pub fn borrow<const POLARITY: Polarity, const PHASE: Phase>(&mut self) -> &mut PayloadSPIPeripheral<POLARITY, PHASE> {
        // Using our knowledge of how PayloadSPIPeripheral works, we can safely convert between types manually, bypassing Rusts's type system (necessary to keep the wrapper free of types).
        // The only thing differentiating PayloadSPIPeripheral<A, B> from PayloadSPIPeripheral<C, D> is
        // a) Internal state
        // b) the methods called on the struct. 

        // We can deal with a) easily enough.
        self.spi_bus.bus.reconfigure(POLARITY, PHASE, BitOrder::MsbFirst, BitCount::EightBits);

        // Now we only need to trick Rust into calling the methods of PayloadSPIPeripheral<POLARITY, PHASE> instead of the methods associated with our PayloadSPIBitBang<{IdleHigh}, {DeviceReadsFirstEdge}> we have stored.        
        // Ask Rust to treat our PayloadSPIPeripheral<{IdleHigh}, {DeviceReadsFirstEdge}> as if it were PayloadSPIPeripheral<POLARITY, PHASE>.
        // This will take care of the rest of the conversion, as Rust will now call the methods associated with PayloadSPIPeripheral<POLARITY, PHASE>.
        // This, combined with the above sck polarity is all that is necessary to convert between the types.
        // As far as I know this is safe, as PayloadSPIController is zero-sized.
        return unsafe{ core::mem::transmute(&mut self.spi_bus) };
    }
}