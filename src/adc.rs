use msp430fr2355::P6;
use msp430fr2x5x_hal::gpio::{*};
use no_std_compat::marker::PhantomData;

use embedded_hal::digital::v2::OutputPin;

use crate::{spi::PayloadSPI};
use crate::pcb_mapping_v5::{ADC_VCC_VOLTAGE_MILLIVOLTS, ISOLATED_ADC_VCC_VOLTAGE_MILLIVOLTS};

#[derive(PartialEq)]
pub enum TargetADC {
	TetherADC,
	TemperatureADC,
	MiscADC,
}
#[derive(PartialEq,Copy,Clone)]
pub enum ADCChannel {
	IN0=0,
	IN1=1,
	IN2=2,
	IN3=3,
	IN4=4,
	IN5=5,
	IN6=6,
	IN7=7,
}

//Types to make sure that we can't read sensor X from ADC Y, because otherwise voltage conversion will be incorrect
pub trait ADCSensor{fn channel(&self) -> ADCChannel;}
pub struct TetherSensor {pub channel: ADCChannel}
impl ADCSensor for TetherSensor{fn channel(&self) -> ADCChannel {self.channel}}
pub struct MiscSensor {pub channel: ADCChannel}
impl ADCSensor for MiscSensor{fn channel(&self) -> ADCChannel {self.channel}}
pub struct TemperatureSensor {pub channel: ADCChannel}
impl ADCSensor for TemperatureSensor{fn channel(&self) -> ADCChannel {self.channel}}

// The ADCs do not all share the same VCC. 
pub struct TetherADC{}
impl TetherADC{
    pub fn new(cs_pin: Pin<P6, Pin2, Output>) -> ADC<P6, Pin2, TetherSensor> {
        ADC::new(ISOLATED_ADC_VCC_VOLTAGE_MILLIVOLTS, cs_pin)
    }
}
pub struct MiscADC{}
impl MiscADC {
    pub fn new(cs_pin: Pin<P5, Pin4, Output>) -> ADC<P5, Pin4, MiscSensor> {
        ADC::new(ADC_VCC_VOLTAGE_MILLIVOLTS, cs_pin)
    }
}
pub struct TemperatureADC{}
impl TemperatureADC {
    pub fn new(cs_pin: Pin<P6, Pin0, Output>) -> ADC<P6, Pin0, TemperatureSensor> {
        ADC::new(ADC_VCC_VOLTAGE_MILLIVOLTS, cs_pin)
    }
}

//let temperature_adc = TemperatureADC::new();
//temperature_adc.read_count_from(TetherSensor{adc:TetherADC, channel:ADCChannel::IN0}) // compile error
//temperature_adc.read_count_from(TemperatureSensor{adc:TemperatureADC, channel:ADCChannel::IN0}) // ok

const ADC_RESOLUTION: u16 = 4095;
pub struct ADC<PORT:PortNum, PIN:PinNum, SensorType:ADCSensor>{
    vcc_millivolts: u16,
    cs_pin: Pin<PORT, PIN, Output>,
    _adc_type: PhantomData<SensorType>
}
impl<PORT:PortNum, PIN:PinNum, SensorType:ADCSensor> ADC<PORT, PIN, SensorType>{
    pub fn new(vcc_millivolts: u16, cs_pin: Pin<PORT, PIN, Output>) -> ADC<PORT, PIN, SensorType
> {
        ADC::<PORT, PIN, SensorType
    >{vcc_millivolts, cs_pin, _adc_type: PhantomData}
    }
    // Note: ADC always sends the value of IN0 when first selected, second reading will be from the channel provided.
    pub fn read_count_from(&mut self, wanted_sensor: &SensorType
    , spi_bus: &mut dyn PayloadSPI) -> u16{
        let result: u16;
        spi_bus.set_sck_idle_high();
        self.cs_pin.set_low().unwrap();
        
        if wanted_sensor.channel() == ADCChannel::IN0 {
            spi_bus.receive(4);
            result = spi_bus.receive(12) as u16;
        }
        else{
            // ADC takes four cycles to track signal. Nothing to do for first two.
            spi_bus.receive(2);

            // Send channel. ADC Sends the first bit of IN0, which we don't care about.
            spi_bus.send(3, wanted_sensor.channel() as u32);

            //Wait out the rest of the IN0 reading being sent to us
            spi_bus.receive(11);

            // ADC is now tracking the channel we want
            spi_bus.receive(4);

            //Finally receive ADC value from the channel we care about
            result = spi_bus.receive(12) as u16;
        }
        self.cs_pin.set_high().unwrap();
        result
    }
    pub fn count_to_voltage(&self, count: u16) -> u16{
        count * self.vcc_millivolts / ADC_RESOLUTION
    }
    pub fn read_voltage_from(&mut self,wanted_sensor: &SensorType
    , spi_bus: &mut dyn PayloadSPI) -> u16{
        let count = self.read_count_from(&wanted_sensor, spi_bus);
        self.count_to_voltage(count)
    }
}