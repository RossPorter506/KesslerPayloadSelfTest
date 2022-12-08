use core::{marker::PhantomData};

use crate::pcb_mapping_v5::{OBCSPIPins, PayloadMisoPin, PayloadMosiPin, PayloadSckPin, PayloadMisoPort, PayloadSckPort, PayloadMosiPort};
use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin, InputPin};
use msp430fr2x5x_hal::gpio::*;
use crate::delay_cycles;

// Trait because we can implement by either bitbanging or using peripheral
// Separate traits befause OBC_SPI might be expanded in the future (e.g. pin interrupts)
// (Plus, it's hell to make the pins generic)
pub trait OBCSPI{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32;
}
pub trait PayloadSPI<Polarity: SckPolarity, Phase: SckPhase>{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32;
}

// Some peripherals expect the bus left high or low when idle, and some read rising edges while others read falling edges.
// Encode this in types so peripherals can enforce a correct configuration
pub trait SckPolarity{}
pub struct IdleHigh; impl SckPolarity for IdleHigh{}
pub struct IdleLow; impl SckPolarity for IdleLow{}
pub struct NoPolaritySet;

pub trait SckPhase{}
pub struct SampleRisingEdge; impl SckPhase for SampleRisingEdge{}
pub struct SampleFallingEdge; impl SckPhase for SampleFallingEdge{}
pub struct NoPhaseSet;

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
// Ex: .new().sck_idle_low().sample_on_rising_edge().create()
pub struct PayloadSPIBitBangConfig<Polarity,Phase>{
    pub miso:   Pin<PayloadMisoPort, PayloadMisoPin, Input<Pulldown>>, 
    pub mosi:   Pin<PayloadMosiPort, PayloadMosiPin, Output>, 
    pub sck:    Pin<PayloadSckPort, PayloadSckPin, Output>, 
    _polarity:  PhantomData<Polarity>,
    _phase:     PhantomData<Phase>,
}
impl PayloadSPIBitBangConfig<NoPolaritySet, NoPhaseSet>{
    pub fn new( miso: Pin<PayloadMisoPort, PayloadMisoPin, Input<Pulldown>>, 
                mosi: Pin<PayloadMosiPort, PayloadMosiPin, Output>, 
                sck: Pin<PayloadSckPort, PayloadSckPin, Output>) -> PayloadSPIBitBangConfig<NoPolaritySet, NoPhaseSet>{
        PayloadSPIBitBangConfig::<NoPolaritySet, NoPhaseSet>{   miso, mosi, sck,
                                                                _polarity: PhantomData,
                                                                _phase: PhantomData, }
    }
}
impl<NoPolaritySet, Phase> PayloadSPIBitBangConfig<NoPolaritySet, Phase>{
    pub fn sck_idle_high(mut self) -> PayloadSPIBitBangConfig<IdleHigh, Phase> {
        self.sck.set_high().ok();
        PayloadSPIBitBangConfig::<IdleHigh, Phase>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
    pub fn sck_idle_low(mut self) -> PayloadSPIBitBangConfig<IdleLow, Phase> {
        self.sck.set_low().ok();
        PayloadSPIBitBangConfig::< IdleLow, Phase>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
impl<Polarity, NoPhaseSet> PayloadSPIBitBangConfig<Polarity, NoPhaseSet>{
    pub fn sample_on_rising_edge(self) -> PayloadSPIBitBangConfig<Polarity, SampleRisingEdge> {
        PayloadSPIBitBangConfig::<Polarity, SampleRisingEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
    pub fn sample_on_falling_edge(self) -> PayloadSPIBitBangConfig<Polarity, SampleFallingEdge> {
        PayloadSPIBitBangConfig::<Polarity, SampleFallingEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
impl<Polarity: SckPolarity, Phase: SckPhase> PayloadSPIBitBangConfig<Polarity, Phase>{
    pub fn create(self) -> PayloadSPIBitBang<Polarity, Phase> {
        PayloadSPIBitBang::<Polarity, Phase>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}

pub struct PayloadSPIBitBang<Polarity: SckPolarity, Phase: SckPhase>{
    pub miso:   Pin<PayloadMisoPort, PayloadMisoPin, Input<Pulldown>>, 
    pub mosi:   Pin<PayloadMosiPort, PayloadMosiPin, Output>, 
    pub sck:    Pin<PayloadSckPort, PayloadSckPin, Output>, 
    _polarity:  PhantomData<Polarity>,
    _phase:     PhantomData<Phase>,
}
//Internal functions to reduce code duplication. (IdleHigh and SampleRising) == (IdleLow and SampleFalling), except the initial state of the clock is inverted. Vice versa for the other pair
impl<Polarity: SckPolarity, Phase: SckPhase> PayloadSPIBitBang<Polarity, Phase>{
    fn receive_on_first_edge(&mut self, len: u8) -> u32 {
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        while current_pos < len {
            self.sck.toggle().ok();
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            delay_cycles(40); // duty cycle correction
            self.sck.toggle().ok();
            current_pos += 1;
        }
        result
    }
    fn receive_on_second_edge(&mut self, len: u8) -> u32 {
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
    fn send_on_first_edge(&mut self, len: u8, data: u32) {
        let mut current_pos: u8 = 0;
        while current_pos < len {
            self.sck.toggle().ok();
            if  (data & (1_u32 << (len - current_pos - 1_u8))) == 1_u32 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            delay_cycles(80); // duty cycle correction
            self.sck.toggle().ok();
            current_pos += 1;
        }
    }
    fn send_on_second_edge(&mut self, len: u8, data: u32) {
        let mut current_pos: u8 = 0;
        while current_pos < len {
            self.sck.toggle().ok();
            delay_cycles(80); // duty cycle correction
            if  (data & (1_u32 << (len - current_pos - 1_u8))) == 1_u32 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            current_pos += 1;
        }
    }
    fn send_on_first_receive_on_second(&mut self, len: u8, data: u32) -> u32{
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
    fn send_on_second_receive_on_first(&mut self, len: u8, data: u32) -> u32{
        let mut result: u32 = 0;
        let mut current_pos: u8 = 0;
        while current_pos < len {
            
            self.sck.toggle().ok();
            result = (result << 1) | (self.miso.is_high().unwrap() as u32);
            delay_cycles(80); // duty cycle correction
            if  (data & (1_u32 << (len - current_pos - 1_u8))) == 1_u32 {
                self.mosi.set_high().ok();
            }
            else{
                self.mosi.set_low().ok();
            }
            self.sck.toggle().ok();
            current_pos += 1;
        }
        result
    }
    pub fn return_pins(self) -> (Pin<PayloadMisoPort, PayloadMisoPin, Input<Pulldown>>, 
                                 Pin<PayloadMosiPort, PayloadMosiPin, Output>, 
                                 Pin<PayloadSckPort, PayloadSckPin, Output>){
        (self.miso, self.mosi, self.sck)
    }
}
// Transformation functions
impl<Phase: SckPhase> PayloadSPIBitBang<IdleHigh, Phase>{
    pub fn into_idle_low(mut self) -> PayloadSPIBitBang<IdleLow, Phase> {
        self.sck.set_low().ok();
        PayloadSPIBitBang::<IdleLow, Phase>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
impl<Phase: SckPhase> PayloadSPIBitBang<IdleLow, Phase>{
    pub fn into_idle_high(mut self) -> PayloadSPIBitBang<IdleHigh, Phase> {
        self.sck.set_high().ok();
        PayloadSPIBitBang::<IdleHigh, Phase>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
impl<Polarity: SckPolarity> PayloadSPIBitBang<Polarity, SampleFallingEdge>{
    pub fn into_sample_rising_edge(self) -> PayloadSPIBitBang<Polarity, SampleRisingEdge> {
        PayloadSPIBitBang::<Polarity, SampleRisingEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
impl<Polarity: SckPolarity> PayloadSPIBitBang<Polarity, SampleRisingEdge>{
    pub fn into_sample_falling_edge(self) -> PayloadSPIBitBang<Polarity, SampleFallingEdge> {
        PayloadSPIBitBang::<Polarity, SampleFallingEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
// Actual implementations
impl PayloadSPI<IdleHigh, SampleRisingEdge> for PayloadSPIBitBang<IdleHigh, SampleRisingEdge> {
    fn send(&mut self, len: u8, data: u32) { self.send_on_first_edge(len, data) }
    fn receive(&mut self, len: u8) -> u32  { self.receive_on_second_edge(len) }
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 { self.send_on_first_receive_on_second(len, data) }
}
impl PayloadSPI<IdleHigh, SampleFallingEdge> for PayloadSPIBitBang<IdleHigh, SampleFallingEdge> {
    fn send(&mut self, len: u8, data: u32) { self.send_on_second_edge(len, data) }
    fn receive(&mut self, len: u8) -> u32  { self.receive_on_first_edge(len) }
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 { self.send_on_second_receive_on_first(len, data) }
}
impl PayloadSPI<IdleLow, SampleRisingEdge> for PayloadSPIBitBang<IdleLow, SampleRisingEdge> {
    fn send(&mut self, len: u8, data: u32) { self.send_on_second_edge(len, data) }
    fn receive(&mut self, len: u8) -> u32  { self.receive_on_first_edge(len) }
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 { self.send_on_second_receive_on_first(len, data) }
}
impl PayloadSPI<IdleLow, SampleFallingEdge> for PayloadSPIBitBang<IdleLow, SampleFallingEdge> {
    fn send(&mut self, len: u8, data: u32) { self.send_on_first_edge(len, data) }
    fn receive(&mut self, len: u8) -> u32  { self.receive_on_second_edge(len) }
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32 { self.send_on_first_receive_on_second(len, data) }
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