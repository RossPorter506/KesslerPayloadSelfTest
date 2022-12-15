// This file interacts with an ADC128S052 Analog to Digital Converter (ADC). 
// It includes generic types to allow for multiple ADCs connected to the same SPI bus. 
// PCB-specific values (e.g. reference voltages, channel connections) can be found in the pcb_mapping file.

use embedded_hal::digital::v2::OutputPin;
use no_std_compat::marker::PhantomData;

use crate::spi::{IdleHigh, SampleFallingEdge};
use crate::{spi::PayloadSPI};
use crate::pcb_mapping_v5::{peripheral_vcc_values::*, pin_name_types::*};

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
// Shorthand type for each ADC instance
pub type TetherADC      = ADC<TetherADCCSPin, TetherSensor>;
pub type TemperatureADC = ADC<TemperatureADCCSPin, TemperatureSensor>;
pub type MiscADC        = ADC<MiscADCCSPin, MiscSensor>;

// Generic ADC chip select pin type 
pub trait ADCCSPin: OutputPin{}
impl ADCCSPin for TetherADCCSPin{}
impl ADCCSPin for TemperatureADCCSPin{}
impl ADCCSPin for MiscADCCSPin{}

//Types to make sure that we can't read sensor X from ADC Y, because otherwise voltage conversion will be incorrect, etc.
pub trait ADCSensor{fn channel(&self) -> ADCChannel;}
pub struct TetherSensor {pub channel: ADCChannel}
impl ADCSensor for TetherSensor{fn channel(&self) -> ADCChannel {self.channel}}
pub struct MiscSensor {pub channel: ADCChannel}
impl ADCSensor for MiscSensor{fn channel(&self) -> ADCChannel {self.channel}}
pub struct TemperatureSensor {pub channel: ADCChannel}
impl ADCSensor for TemperatureSensor{fn channel(&self) -> ADCChannel {self.channel}}

//let temperature_adc = TemperatureADC::new();
//temperature_adc.read_count_from(TemperatureSensor{adc:TemperatureADC, channel:ADCChannel::IN0}) // ok
//temperature_adc.read_count_from(TetherSensor{adc:TetherADC, channel:ADCChannel::IN0}) // compile error!

const ADC_RESOLUTION: u16 = 4095;
pub struct ADC<CsPin: ADCCSPin, SensorType:ADCSensor>{
    pub vcc_millivolts: u16,
    pub cs_pin: CsPin,
    _adc_type: PhantomData<SensorType>
}
impl TetherADC{
    pub fn new(cs_pin: TetherADCCSPin) -> TetherADC {
        ADC::<TetherADCCSPin, TetherSensor>{vcc_millivolts: ISOLATED_ADC_VCC_VOLTAGE_MILLIVOLTS, cs_pin, _adc_type: PhantomData}
    }
}
impl TemperatureADC{
    pub fn new(cs_pin: TemperatureADCCSPin) -> TemperatureADC {
        ADC::<TemperatureADCCSPin, TemperatureSensor>{vcc_millivolts: ADC_VCC_VOLTAGE_MILLIVOLTS, cs_pin, _adc_type: PhantomData}
    }
}
impl MiscADC{
    pub fn new(cs_pin: MiscADCCSPin) -> MiscADC {
        ADC::<MiscADCCSPin, MiscSensor>{vcc_millivolts: ADC_VCC_VOLTAGE_MILLIVOLTS, cs_pin, _adc_type: PhantomData}
    }
}
impl<CsPin: ADCCSPin, SensorType:ADCSensor> ADC<CsPin, SensorType>{
    // Note: ADC always sends the value of IN0 when first selected, second reading will be from the channel provided.
    pub fn read_count_from(&mut self, wanted_sensor: &SensorType, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> u16{
        let result: u16;
        let _ = self.cs_pin.set_low();
        
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
        let _ = self.cs_pin.set_high();
        result
    }
    pub fn count_to_voltage(&self, count: u16) -> u16{
        count * self.vcc_millivolts / ADC_RESOLUTION
    }
    pub fn read_voltage_from(&mut self, wanted_sensor: &SensorType, spi_bus: &mut impl PayloadSPI<IdleHigh, SampleFallingEdge>) -> u16{
        let count = self.read_count_from(&wanted_sensor, spi_bus);
        self.count_to_voltage(count)
    }
}