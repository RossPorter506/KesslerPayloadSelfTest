// This file interacts with an ADC128S052 Analog to Digital Converter (ADC). 
// It includes generic types to allow for multiple ADCs connected to the same SPI bus. 
// PCB-specific values (e.g. reference voltages, channel connections) can be found in the pcb_mapping file.

use embedded_hal::digital::v2::OutputPin;
use no_std_compat::marker::PhantomData;

use crate::spi::{Polarity::IdleHigh, Phase::CaptureOnSecondEdge};
use crate::{spi::PayloadSPI};
use crate::pcb_mapping::{peripheral_vcc_values::*, pin_name_types::*};

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
pub struct TemperatureSensor {pub channel: ADCChannel, pub vcc: VccType}
impl ADCSensor for TemperatureSensor{fn channel(&self) -> ADCChannel {self.channel}}

pub enum VccType {
    LMS, // 3V3
    Payload, // 5V0
}

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

const AQUIRE_CYCLES: u8 = 4;
const TRANSMIT_CYCLES: u8 = 12;
pub const NUM_CYCLES_FOR_ONE_READING: u8 = AQUIRE_CYCLES + TRANSMIT_CYCLES;
pub const NUM_CYCLES_FOR_TWO_READINGS: u8 = NUM_CYCLES_FOR_ONE_READING * 2;

pub const NUM_ADDRESS_BITS: u8 = 3;
pub const NUM_LEADING_ZEROES: u8 = 2;

impl<CsPin: ADCCSPin, SensorType:ADCSensor> ADC<CsPin, SensorType>{
    // Note: ADC always sends the value of IN0 when first selected, second reading will be from the channel provided.
    pub fn read_count_from(&mut self, wanted_sensor: &SensorType, spi_bus: &mut impl PayloadSPI<{IdleHigh}, {CaptureOnSecondEdge}>) -> u16{
        // When SPI packet begins the ADC will track and read channel 1 regardless. 
        // If we want another channel we have to wait until it's finished sending this.
        if wanted_sensor.channel() == ADCChannel::IN0 {
            return spi_bus.receive(NUM_CYCLES_FOR_ONE_READING, &mut self.cs_pin) as u16;
        }
        else{
            // We need to send the channel we want to read two edges after the start, and it's three bits long.
            // SPI will always send the LSB during the last edge, so we need to shift it until there are only two zeroes in front, i.e. 00XXX0000...
            // 1 << 31 would put the one-bit-long payload in the MSB, so shift by two fewer for a three-bit payload, and two fewer again to have two zeroes out front
            let data_packet = (wanted_sensor.channel() as u32) << (NUM_CYCLES_FOR_TWO_READINGS - NUM_ADDRESS_BITS - NUM_LEADING_ZEROES);

            let result = spi_bus.send_receive(NUM_CYCLES_FOR_TWO_READINGS, data_packet, &mut self.cs_pin);
            return (result & 0xFFF) as u16; // We only care about the last reading, which is transmitted in the last 12 edges.
        }
    }
    pub fn count_to_voltage(&self, count: u16) -> u16{
        ((count as u32 * self.vcc_millivolts as u32) / ADC_RESOLUTION as u32) as u16
    }
    pub fn read_voltage_from(&mut self, wanted_sensor: &SensorType, spi_bus: &mut impl PayloadSPI<{IdleHigh}, {CaptureOnSecondEdge}>) -> u16{
        let count = self.read_count_from(wanted_sensor, spi_bus);
        self.count_to_voltage(count)
    }
}