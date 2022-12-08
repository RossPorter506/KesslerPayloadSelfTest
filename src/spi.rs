use core::marker::PhantomData;

use crate::pcb_mapping_v5::{PayloadSPIPins, OBCSPIPins};
use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin, InputPin};
use msp430fr2x5x_hal::gpio::*;
use crate::delay_cycles;

// Trait because we can implement by either bitbanging or using peripheral
// Separate traits befause OBC_SPI might be expanded in the future (e.g. pin interrupts)
pub trait OBCSPI{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32;
    fn set_sck_idle_low(&mut self);
    fn set_sck_idle_high(&mut self);
}
pub trait PayloadSPI<IdleType: SckIdleType>{
    fn send(&mut self, len: u8, data: u32);
    fn receive(&mut self, len: u8) -> u32;
    fn send_and_receive(&mut self, len: u8, data: u32) -> u32;
}

// Some peripherals read rising edges while others read falling edges.
// Encode this in a type so we can't pass 
pub trait SckIdleType{}
pub struct SckIdleHigh; impl SckIdleType for SckIdleHigh{}
pub struct SckIdleLow; impl SckIdleType for SckIdleLow{}

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
    fn set_sck_idle_low(&mut self){
        self.sck.set_low().ok();
    }
    fn set_sck_idle_high(&mut self){
        self.sck.set_high().ok();
    }
}

pub struct PayloadSPIBitBang<IdleType: SckIdleType>{
    pub miso:   Pin<P4, Pin7, Input<Pullup>>, 
    pub mosi:   Pin<P4, Pin6, Output>, 
    pub sck:    Pin<P4, Pin5, Output>, 
    _idle_type: PhantomData<IdleType>,
}
impl<IdleType: SckIdleType> PayloadSPIBitBang<IdleType> {
    pub fn new_idle_high_bus(pins: PayloadSPIPins) -> PayloadSPIBitBang<SckIdleHigh>{
        PayloadSPIBitBang::<SckIdleHigh>{  
            miso: pins.miso.to_gpio().to_input_pullup(),
            mosi: pins.mosi.to_gpio(),
            sck:  pins.sck.to_gpio(),
            _idle_type: PhantomData,}
    }
    pub fn new_idle_low_bus(pins: PayloadSPIPins) -> PayloadSPIBitBang<SckIdleLow>{
        PayloadSPIBitBang::<SckIdleLow>{  
            miso: pins.miso.to_gpio().to_input_pullup(),
            mosi: pins.mosi.to_gpio(),
            sck:  pins.sck.to_gpio(),
            _idle_type: PhantomData,}
    }
    pub fn return_pins(self) -> PayloadSPIPins{
        PayloadSPIPins{  miso: self.miso.to_output().to_alternate1(), 
                         mosi: self.mosi.to_alternate1(), 
                         sck: self.sck.to_alternate1()}
    }
    pub fn into_sck_idle_low(mut self) -> PayloadSPIBitBang<SckIdleLow>{
        self.sck.set_low().ok();
        PayloadSPIBitBang::<SckIdleLow>{mosi: self.mosi, miso: self.miso, sck: self.sck, _idle_type: PhantomData}
    }
    pub fn into_sck_idle_high(mut self) -> PayloadSPIBitBang<SckIdleHigh>{
        self.sck.set_high().ok();
        PayloadSPIBitBang::<SckIdleHigh>{mosi: self.mosi, miso: self.miso, sck: self.sck, _idle_type: PhantomData}
    }
}
impl<IdleType:SckIdleType> PayloadSPI<IdleType> for PayloadSPIBitBang<IdleType> {
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