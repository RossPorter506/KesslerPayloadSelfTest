use core::{marker::PhantomData};

use crate::pcb_mapping_v5::{OBCSPIPins, PayloadSPIPins, pin_name_types::{PayloadMOSIBitBangPin, PayloadMISOBitBangPin, PayloadSCKBitBangPin}, PayloadSPIBitBangPins};
use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin, InputPin};
use msp430fr2x5x_hal::gpio::*;
use crate::delay_cycles;

// Trait because we can implement by either bitbanging or using peripheral
// Separate traits befause OBC_SPI might be expanded in the future (e.g. pin interrupts)
pub trait OBCSPI{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32;
}
pub trait PayloadSPI<Polarity: SckPolarity, Phase: SckPhase>{
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin);
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32;
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32;
}

// Some peripherals expect the bus left high or low when idle, and some read rising edges while others read falling edges.
// Encode this in types so peripherals can enforce a correct configuration
pub trait SckPolarity{}
pub struct IdleHigh; impl SckPolarity for IdleHigh{}
pub struct IdleLow; impl SckPolarity for IdleLow{}
pub struct NoPolaritySet;

pub trait SckPhase{}
pub struct SampleFirstEdge; impl SckPhase for SampleFirstEdge{}
pub struct SampleSecondEdge; impl SckPhase for SampleSecondEdge{}
pub struct NoPhaseSet;

pub trait SpiType{}
pub struct BitBang; impl SpiType for BitBang {}
pub struct Peripheral; impl SpiType for Peripheral {}
pub struct NoTypeSet;

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
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 {
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

// Constructor for PayloadSPI
// Ex: .new().sck_idle_low().sample_on_first_edge().bit_bang().create()
// All peripherals use the 'sample on first edge' phase, but it doesn't hurt to have the second edge stuff.
pub struct PayloadSPIConfig<Polarity,Phase,Type>{
    pub miso:   PayloadMISOBitBangPin, 
    pub mosi:   PayloadMOSIBitBangPin, 
    pub sck:    PayloadSCKBitBangPin, 
    _polarity:  PhantomData<Polarity>,
    _phase:     PhantomData<Phase>,
    _type:     PhantomData<Type>,
}
impl PayloadSPIConfig<NoPolaritySet, NoPhaseSet, NoTypeSet>{
    pub fn new_from_pins( miso: PayloadMISOBitBangPin, 
        mosi: PayloadMOSIBitBangPin, 
        sck: PayloadSCKBitBangPin) -> PayloadSPIConfig<NoPolaritySet, NoPhaseSet, NoTypeSet>{
        
        PayloadSPIConfig::<NoPolaritySet, NoPhaseSet, NoTypeSet>{   miso, mosi, sck,
            _polarity: PhantomData,
            _phase: PhantomData, 
            _type: PhantomData}
    }
    pub fn new_from_struct( pins: PayloadSPIBitBangPins) -> PayloadSPIConfig<NoPolaritySet, NoPhaseSet, NoTypeSet>{
        PayloadSPIConfig::<NoPolaritySet, NoPhaseSet, NoTypeSet>{   
            miso: pins.miso, 
            mosi: pins.mosi, 
            sck: pins.sck,
            _polarity: PhantomData,
            _phase: PhantomData,
            _type: PhantomData }
    }
}

// These functions may be called when polarity has not been set. Others can have any value (set, not set, etc.)
impl<NoPolaritySet, Phase, Type> PayloadSPIConfig<NoPolaritySet, Phase, Type>{
    pub fn sck_idle_high(mut self) -> PayloadSPIConfig<IdleHigh, Phase, Type> {
        self.sck.set_high().ok();
        PayloadSPIConfig::<IdleHigh, Phase, Type>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
    pub fn sck_idle_low(mut self) -> PayloadSPIConfig<IdleLow, Phase, Type> {
        self.sck.set_low().ok();
        PayloadSPIConfig::< IdleLow, Phase, Type>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
// These functions may be called when phase has not been set. Others can have any value (set, not set, etc.)
impl<Polarity, NoPhaseSet, Type> PayloadSPIConfig<Polarity, NoPhaseSet, Type>{
    pub fn sample_on_first_edge(self) -> PayloadSPIConfig<Polarity, SampleFirstEdge, Type> {
        PayloadSPIConfig::<Polarity, SampleFirstEdge, Type>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
    pub fn sample_on_second_edge(self) -> PayloadSPIConfig<Polarity, SampleSecondEdge, Type> {
        PayloadSPIConfig::<Polarity, SampleSecondEdge, Type>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
// These functions may be called when type has not been set. Others can have any value (set, not set, etc.)
impl<Polarity, Phase, NoTypeSet> PayloadSPIConfig<Polarity, Phase, NoTypeSet>{
    pub fn bitbang(self) -> PayloadSPIConfig<Polarity, Phase, BitBang> {
        PayloadSPIConfig::<Polarity, Phase, BitBang>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
    pub fn peripheral(self) -> PayloadSPIConfig<Polarity, Phase, Peripheral> {
        PayloadSPIConfig::<Polarity, Phase, Peripheral>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
// These functions can only be called when all have been set.
impl<Polarity: SckPolarity, Phase: SckPhase, Type: SpiType> PayloadSPIConfig<Polarity, Phase, Type>{
    pub fn create(self) -> PayloadSPIBus<Polarity, Phase, Type> {
        PayloadSPIBus::<Polarity, Phase, Type>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}

pub struct PayloadSPIBus<Polarity: SckPolarity, Phase: SckPhase, Type: SpiType>{
    pub miso:   PayloadMISOBitBangPin, 
    pub mosi:   PayloadMOSIBitBangPin, 
    pub sck:    PayloadSCKBitBangPin, 
    _polarity:  PhantomData<Polarity>,
    _phase:     PhantomData<Phase>,
    _type:     PhantomData<Type>,
}

//Internal functions to reduce code duplication. (IdleHigh and SampleRising) == (IdleLow and SampleFalling), except the initial state of the clock is inverted. Vice versa for the other pair
//Could combine each pair into one function, but I don't want branches inside the main bitbang loop, as bitbanging is already slow enough.
impl<Polarity: SckPolarity, Phase: SckPhase> PayloadSPIBus<Polarity, Phase, BitBang>{
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
}
// Transformation functions
impl<Phase: SckPhase> PayloadSPIBus<IdleHigh, Phase, BitBang>{
    pub fn into_idle_low(mut self) -> PayloadSPIBus<IdleLow, Phase, BitBang> {
        self.sck.set_low().ok();
        PayloadSPIBus::<IdleLow, Phase, BitBang>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
impl<Phase: SckPhase> PayloadSPIBus<IdleLow, Phase, BitBang>{
    pub fn into_idle_high(mut self) -> PayloadSPIBus<IdleHigh, Phase, BitBang> {
        self.sck.set_high().ok();
        PayloadSPIBus::<IdleHigh, Phase, BitBang>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
impl<Polarity: SckPolarity> PayloadSPIBus<Polarity, SampleSecondEdge, BitBang>{
    pub fn into_sample_first_edge(self) -> PayloadSPIBus<Polarity, SampleFirstEdge, BitBang> {
        PayloadSPIBus::<Polarity, SampleFirstEdge, BitBang>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
impl<Polarity: SckPolarity> PayloadSPIBus<Polarity, SampleFirstEdge, BitBang>{
    pub fn into_sample_second_edge(self) -> PayloadSPIBus<Polarity, SampleSecondEdge, BitBang> {
        PayloadSPIBus::<Polarity, SampleSecondEdge, BitBang>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData, _type: PhantomData}
    }
}
// Actual trait implementations
impl PayloadSPI<IdleHigh, SampleSecondEdge> for PayloadSPIBus<IdleHigh, SampleSecondEdge, BitBang> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_first_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_second_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_first_receive_on_second(len, data, cs_pin) }
}
impl PayloadSPI<IdleHigh, SampleFirstEdge> for PayloadSPIBus<IdleHigh, SampleFirstEdge, BitBang> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_second_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_first_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_second_receive_on_first(len, data, cs_pin) }
}
impl PayloadSPI<IdleLow, SampleFirstEdge> for PayloadSPIBus<IdleLow, SampleFirstEdge, BitBang> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_second_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_first_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_second_receive_on_first(len, data, cs_pin) }
}
impl PayloadSPI<IdleLow, SampleSecondEdge> for PayloadSPIBus<IdleLow, SampleSecondEdge, BitBang> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_first_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_second_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_first_receive_on_second(len, data, cs_pin) }
}

impl<Phase: SckPhase, Polarity: SckPolarity> PayloadSPI<Polarity, Phase> for PayloadSPIBus<Polarity, Phase, Peripheral> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { todo!() }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { todo!() }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 {  todo!() }
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