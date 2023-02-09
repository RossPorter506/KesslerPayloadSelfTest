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
// Ex: .new().sck_idle_low().sample_on_first_edge().create()
// All peripherals use the 'sample on first edge' phase, but it doesn't hurt to have the second edge stuff.
pub struct PayloadSPIBitBangConfig<Polarity,Phase>{
    pub miso:   PayloadMISOBitBangPin, 
    pub mosi:   PayloadMOSIBitBangPin, 
    pub sck:    PayloadSCKBitBangPin, 
    _polarity:  PhantomData<Polarity>,
    _phase:     PhantomData<Phase>,
}
impl PayloadSPIBitBangConfig<NoPolaritySet, NoPhaseSet>{
    pub fn new_from_pins( miso: PayloadMISOBitBangPin, 
                mosi: PayloadMOSIBitBangPin, 
                sck: PayloadSCKBitBangPin) -> PayloadSPIBitBangConfig<NoPolaritySet, NoPhaseSet>{
        PayloadSPIBitBangConfig::<NoPolaritySet, NoPhaseSet>{   miso, mosi, sck,
                                                                _polarity: PhantomData,
                                                                _phase: PhantomData, }
    }
    pub fn new_from_struct( pins: PayloadSPIBitBangPins) -> PayloadSPIBitBangConfig<NoPolaritySet, NoPhaseSet>{
        PayloadSPIBitBangConfig::<NoPolaritySet, NoPhaseSet>{   
            miso: pins.miso, 
            mosi: pins.mosi, 
            sck: pins.sck,
            _polarity: PhantomData,
            _phase: PhantomData, }
    }
}
// These functions may be called when polarity has not been set. Phase can have any value (set, not set, etc.)
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
// These functions may be called when phase has not been set. Polarity can have any value (set, not set, etc.)
impl<Polarity, NoPhaseSet> PayloadSPIBitBangConfig<Polarity, NoPhaseSet>{
    pub fn sample_on_first_edge(self) -> PayloadSPIBitBangConfig<Polarity, SampleFirstEdge> {
        PayloadSPIBitBangConfig::<Polarity, SampleFirstEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
    pub fn sample_on_second_edge(self) -> PayloadSPIBitBangConfig<Polarity, SampleSecondEdge> {
        PayloadSPIBitBangConfig::<Polarity, SampleSecondEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
// These functions can only be called when both phase and polarity have been set.
impl<Polarity: SckPolarity, Phase: SckPhase> PayloadSPIBitBangConfig<Polarity, Phase>{
    pub fn create(self) -> PayloadSPIBitBang<Polarity, Phase> {
        PayloadSPIBitBang::<Polarity, Phase>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}

pub struct PayloadSPIBitBang<Polarity: SckPolarity, Phase: SckPhase>{
    pub miso:   PayloadMISOBitBangPin, 
    pub mosi:   PayloadMOSIBitBangPin, 
    pub sck:    PayloadSCKBitBangPin, 
    _polarity:  PhantomData<Polarity>,
    _phase:     PhantomData<Phase>,
}

//Internal functions to reduce code duplication. (IdleHigh and SampleRising) == (IdleLow and SampleFalling), except the initial state of the clock is inverted. Vice versa for the other pair
//Could combine each pair into one function, but I don't want branches inside the main bitbang loop, as bitbanging is already slow enough.
impl<Polarity: SckPolarity, Phase: SckPhase> PayloadSPIBitBang<Polarity, Phase>{
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
impl<Polarity: SckPolarity> PayloadSPIBitBang<Polarity, SampleSecondEdge>{
    pub fn into_sample_first_edge(self) -> PayloadSPIBitBang<Polarity, SampleFirstEdge> {
        PayloadSPIBitBang::<Polarity, SampleFirstEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
impl<Polarity: SckPolarity> PayloadSPIBitBang<Polarity, SampleFirstEdge>{
    pub fn into_sample_second_edge(self) -> PayloadSPIBitBang<Polarity, SampleSecondEdge> {
        PayloadSPIBitBang::<Polarity, SampleSecondEdge>{miso: self.miso, mosi: self.mosi, sck: self.sck, _polarity: PhantomData, _phase: PhantomData}
    }
}
// Actual trait implementations
impl PayloadSPI<IdleHigh, SampleSecondEdge> for PayloadSPIBitBang<IdleHigh, SampleSecondEdge> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_first_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_second_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_first_receive_on_second(len, data, cs_pin) }
}
impl PayloadSPI<IdleHigh, SampleFirstEdge> for PayloadSPIBitBang<IdleHigh, SampleFirstEdge> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_second_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_first_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_second_receive_on_first(len, data, cs_pin) }
}
impl PayloadSPI<IdleLow, SampleFirstEdge> for PayloadSPIBitBang<IdleLow, SampleFirstEdge> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_second_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_first_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_second_receive_on_first(len, data, cs_pin) }
}
impl PayloadSPI<IdleLow, SampleSecondEdge> for PayloadSPIBitBang<IdleLow, SampleSecondEdge> {
    fn send(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) { self.send_on_first_edge(len, data, cs_pin) }
    fn receive(&mut self, len: u8, cs_pin: &mut impl OutputPin) -> u32  { self.receive_on_second_edge(len, cs_pin) }
    fn send_and_receive(&mut self, len: u8, data: u32, cs_pin: &mut impl OutputPin) -> u32 { self.send_on_first_receive_on_second(len, data, cs_pin) }
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