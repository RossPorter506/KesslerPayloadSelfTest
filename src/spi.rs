use crate::pcb_mapping::{OBCSPIPins, PayloadSPIPins, pin_name_types::{PayloadMOSIBitBangPin, PayloadMISOBitBangPin, PayloadSCKBitBangPin}, PayloadSPIBitBangPins};
use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin, InputPin};
use msp430fr2x5x_hal::gpio::*;
use crate::delay_cycles;

// Trait because we can implement by either bitbanging or using peripheral
// Separate traits befause OBC_SPI might be expanded in the future (e.g. pin interrupts)
pub trait OBCSPI{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_receive(&mut self, len: u8, data: u32) -> u32;
}
pub trait PayloadSPI<const POLARITY: SckPolarity, const PHASE: SckPhase> : Sized{
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin);
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32;
    fn send_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32;
}

// Some peripherals expect the bus left high or low when idle, and some read rising edges while others read falling edges.
// Encode this in types so peripherals can enforce a correct configuration
#[derive(PartialEq, Eq)]
pub enum SckPolarity {
    IdleHigh,
    IdleLow,
}
use SckPolarity::*;
#[derive(PartialEq, Eq)]
pub enum SckPhase {
    SampleFirstEdge,
    SampleSecondEdge,
}
use SckPhase::*;

pub struct OBCSPIBitBang{
    pub miso:   Pin<P4, Pin2, Input<Pulldown>>, 
    pub mosi:   Pin<P4, Pin3, Output>, 
    pub sck:    Pin<P4, Pin1, Output>, 
    _chip_select:            Pin<P4, Pin0, Alternate1<Output>>, //direction is DontCare
    _chip_select_interrupt:  Pin<P2, Pin0, Input<Pullup>>, 
}
impl OBCSPIBitBang {
    pub fn new(pins: OBCSPIPins) -> OBCSPIBitBang {
        OBCSPIBitBang{  miso: pins.miso.to_gpio().to_input_pulldown(),
                        mosi: pins.mosi.to_gpio(),
                        sck:  pins.sck.to_gpio(),
                        _chip_select: pins.chip_select,
                        _chip_select_interrupt: pins.chip_select_interrupt,
        }
    }
    pub fn return_pins(self) -> OBCSPIPins {
        OBCSPIPins{ miso: self.miso.to_output().to_alternate1(), 
                    mosi: self.mosi.to_alternate1(), 
                    sck: self.sck.to_alternate1(), 
                    chip_select: self._chip_select, 
                    chip_select_interrupt: self._chip_select_interrupt}
    }
    fn set_sck_idle_low(&mut self){
        self.sck.set_low().ok();
    }
    fn set_sck_idle_high(&mut self){
        self.sck.set_high().ok();
    }
}
impl OBCSPI for OBCSPIBitBang {
    fn send(&mut self, len: u8, data: u32) {
        let mut current_pos: u8 = 0;
        while current_pos < len {
            if  (data & (1_u32 << (len - current_pos - 1_u8))) == 1_u32 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            self.sck.toggle().ok();
            current_pos += 1;
        }
    }
    fn receive(&mut self, len: u8) -> u32 {
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        while current_pos < len {
            self.sck.toggle().ok();
            delay_cycles(40); // duty cycle correction
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            self.sck.toggle().ok();
            current_pos += 1;
        }
        result
    }
    fn send_receive(&mut self, len: u8, data: u32) -> u32 {
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        while current_pos < len {
            if  (data & (1_u32 << (len - current_pos - 1_u8))) == 1_u32 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            self.sck.toggle().ok();
            current_pos += 1;
        }
        result
    }
}

pub struct PayloadSPIBitBang<const POLARITY: SckPolarity, const PHASE: SckPhase>{
    pub miso:   PayloadMISOBitBangPin, 
    pub mosi:   PayloadMOSIBitBangPin, 
    pub sck:    PayloadSCKBitBangPin,
}

//Internal functions to reduce code duplication. (IdleHigh and SampleRising) == (IdleLow and SampleFalling), except the initial state of the clock is inverted. Vice versa for the other pair
//Could combine each pair into one function, but I don't want branches inside the main bitbang loop, as bitbanging is already slow enough.
impl<const POLARITY: SckPolarity, const Phase: SckPhase> PayloadSPIBitBang<POLARITY, Phase>{
    pub fn new(mut pins: PayloadSPIBitBangPins) -> Self {
        match POLARITY {
            IdleHigh => pins.sck.set_high().ok(),
            IdleLow => pins.sck.set_low().ok(),
        };
        Self {miso: pins.miso, mosi:pins.mosi, sck:pins.sck}
    }
    fn receive_on_first_edge(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32 {
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        cs_pin.set_low().ok();
        while current_pos < len {
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            self.sck.toggle().ok();
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            current_pos += 1;
        }
        cs_pin.set_high().ok();
        result
    }
    fn receive_on_second_edge(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32 {
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        cs_pin.set_low().ok();
        while current_pos < len {
            self.sck.toggle().ok();
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            current_pos += 1;
        }
        cs_pin.set_high().ok();
        result
    }
    fn send_on_first_edge(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) {
        let mut current_pos: u8 = 0;
        cs_pin.set_low().ok();
        while current_pos < len {
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            if  (data & (1_u32 << (len - current_pos - 1_u8))) > 0 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            current_pos += 1;
        }
        cs_pin.set_high().ok();
    }
    fn send_on_second_edge(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) {
        let mut current_pos: u8 = 0;
        cs_pin.set_low().ok();
        while current_pos < len {
            if (data & (1_u32 << (len - current_pos - 1_u8))) > 0 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            self.sck.toggle().ok();
            current_pos += 1;
        }
        cs_pin.set_high().ok();
    }
    fn send_on_first_receive_on_second(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32{
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        cs_pin.set_low().ok();
        while current_pos < len {
            self.sck.toggle().ok();
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            delay_cycles(80); // duty cycle correction
            if  (data & (1_u32 << (len - current_pos - 1_u8))) > 0 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            current_pos += 1;
        }
        cs_pin.set_high().ok();
        result
    }
    fn send_on_second_receive_on_first(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32{
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        cs_pin.set_low().ok();
        while current_pos < len {
            
            if  (data & (1_u32 << (len - current_pos - 1_u8))) > 0 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            self.sck.toggle().ok();
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            current_pos += 1;
        }
        cs_pin.set_high().ok();
        result
    }
    pub fn return_pins(self) -> PayloadSPIPins {
        PayloadSPIPins{miso:self.miso.to_output().to_alternate1(), mosi:self.mosi.to_alternate1(), sck:self.sck.to_alternate1()}
    }
    pub fn return_bit_bang_pins(self) -> PayloadSPIBitBangPins {
        PayloadSPIBitBangPins{miso:self.miso, mosi:self.mosi, sck:self.sck}
    }
}
// Transformation functions
impl<const CURRENT_POL: SckPolarity, const CURRENT_PHA: SckPhase> PayloadSPIBitBang<CURRENT_POL, CURRENT_PHA> {
    pub fn into<const NEW_POL: SckPolarity, const NEW_PHA: SckPhase>(mut self) -> PayloadSPIBitBang<NEW_POL, NEW_PHA>{
        match NEW_POL {
            IdleHigh => self.sck.set_high().ok(),
            IdleLow => self.sck.set_low().ok(),
        };

        PayloadSPIBitBang::<NEW_POL, NEW_PHA>{miso: self.miso, mosi: self.mosi, sck: self.sck}
    }
}
// Actual trait implementations
impl PayloadSPI<{IdleHigh}, {SampleSecondEdge}> for PayloadSPIBitBang<{IdleHigh}, {SampleSecondEdge}> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_first_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_second_edge(len, cs_pin) }
    fn send_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_first_receive_on_second(len, data, cs_pin) }
}
impl PayloadSPI<{IdleHigh}, {SampleFirstEdge}> for PayloadSPIBitBang<{IdleHigh}, {SampleFirstEdge}> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_second_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_first_edge(len, cs_pin) }
    fn send_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_second_receive_on_first(len, data, cs_pin) }
}
impl PayloadSPI<{IdleLow}, {SampleFirstEdge}> for PayloadSPIBitBang<{IdleLow}, {SampleFirstEdge}> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_second_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_first_edge(len, cs_pin) }
    fn send_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_second_receive_on_first(len, data, cs_pin) }
}
impl PayloadSPI<{IdleLow}, {SampleSecondEdge}> for PayloadSPIBitBang<{IdleLow}, {SampleSecondEdge}> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_first_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_second_edge(len, cs_pin) }
    fn send_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_first_receive_on_second(len, data, cs_pin) }
}

// A wrapper class that automates changing the state of the bus. Useful for intermediate functions that don't use the bus themselves, but call functions that do.
// Functions that require the SPI bus can borrow it using .borrow()
pub struct PayloadSPIController {
    // It looks like we can only store one type here, but we discard type information before storing.
    // Trust me, this is the easiest way.
    spi_bus: PayloadSPIBitBang<{IdleHigh}, {SampleFirstEdge}>
}
impl PayloadSPIController {
    pub fn new_from_bus<const POLARITY: SckPolarity, const PHASE: SckPhase>(bus: PayloadSPIBitBang<POLARITY, PHASE>) -> Self {
        Self {spi_bus: bus.into()}
    }
    pub fn new<const POLARITY: SckPolarity, const PHASE: SckPhase>(pins: PayloadSPIBitBangPins) -> Self {
        let bus = PayloadSPIBitBang::<{IdleHigh}, {SampleFirstEdge}>::new(pins);
        Self {spi_bus: bus.into()}
    }
    pub fn return_bus<const POLARITY: SckPolarity, const PHASE: SckPhase>(self) -> PayloadSPIBitBang<POLARITY, PHASE> {
        self.spi_bus.into()
    }
    pub fn return_pins<const POLARITY: SckPolarity, const PHASE: SckPhase>(self) -> PayloadSPIBitBangPins {
        self.spi_bus.return_bit_bang_pins()
    }
    
    pub fn borrow<const POLARITY: SckPolarity, const PHASE: SckPhase>(&mut self) -> &mut PayloadSPIBitBang<POLARITY, PHASE> {
        // Using our knowledge of how PayloadSPIBitBang works, we can safely convert between types manually, bypassing Rusts's type system (necessary to keep the wrapper free of types).
        // The only thing differentiating PayloadSPIBitBang<A, B> from PayloadSPIBitBang<C, D> is
        // a) Internal state (just the state of the clock pin in our case)
        // b) the methods called on the struct. 

        // We can deal with a) easily enough.
        match POLARITY {
            IdleHigh => self.spi_bus.sck.set_high().ok(),
            IdleLow =>  self.spi_bus.sck.set_low().ok(),
        };

        // Now we only need to trick Rust into calling the methods of PayloadSPIBitBang<POLARITY, PHASE> instead of the methods associated with our PayloadSPIBitBang<{IdleHigh}, {DeviceReadsFirstEdge}> we have stored.
        // Get a pointer to the bus
        let inner_ptr: *mut PayloadSPIBitBang<{IdleHigh}, {SampleFirstEdge}> = &mut self.spi_bus;

        // Ask Rust to treat our PayloadSPIBitBang<{IdleHigh}, {DeviceReadsFirstEdge}> as if it were PayloadSPIBitBang<POLARITY, PHASE>.
        // This will take care of the rest of the conversion, as Rust will now call the methods associated with PayloadSPIBitBang<POLARITY, PHASE>.
        // This, combined with the above sck polarity is all that is necessary to convert between the types.
        let inner_converted_ptr: *mut PayloadSPIBitBang<POLARITY, PHASE> = unsafe{core::mem::transmute(inner_ptr)};

        //...aand return a mutable reference to our bus
        unsafe { &mut *inner_converted_ptr }
    }
}
/*
struct OBCSPIPeripheral{
    pins: OBCSPIPins
}
impl OBCSPIPeripheral {
    fn new(&mut self, pins: OBCSPIPins) {
        self.pins = pins;
    }
    fn return_pins(self) -> OBCSPIPins{
        self.pins
    }
}
impl OBCSPI for OBCSPIPeripheral {
    fn send(&mut self, len: u8, data: u32) {
        
    }
    fn receive(&mut self, len: u8) -> u32 {
        
    }
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 {
        
    }
}
*/

/*
struct PayloadSPIPeripheral{
    pins: PeripheralSPIPins
}
impl PayloadSPIPeripheral {
    fn new(&mut self, pins: PeripheralSPIPins) {
        self.pins = PeripheralSPIPins;
    }
    fn return_pins(self) -> PeripheralSPIPins{
        self.pins
    }
}
impl PayloadSPI for PayloadSPIPeripheral {
    fn send(&mut self, len: u8, data: u32) {

    }
    fn receive(&mut self, len: u8) -> u32 {

    }
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 {

    }
}
*/