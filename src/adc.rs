enum TargetADC {
	TetherADC,
	TemperatureADC,
	MiscADC,
}

enum ADCChannel {
	IN0=0,
	IN1=1,
	IN2=2,
	IN3=3,
	IN4=4,
	IN5=5,
	IN6=6,
	IN7=7,
}

struct ADCSensor {
	adc: TargetADC,
	channel: ADCChannel,
}

// Note: ADC always sends the value of IN0 when first selected, second reading will be from the channel provided.
fn read_count_from(adc_sensor: &ADCSensor, 
                   spi_bus: &mut dyn PeripheralSPI, cs_pins: &mut PeripheralSPIChipSelectPins){
    spi_bus.set_sck_idle_high();
    match adc_sensor {
        TemperatureADC => cs_pins.board_temperature_adc.set_low(),
        TetherADC => cs_pins.tether_measurement_adc.set_low(),
        MiscADC => cs_pins.misc_adc.set_low(),
    }
    if sensor.channel == IN0 {
        spi_bus.receive(4);
        let result = spi_bus.receive(12);
    }
    else{
        // ADC takes four cycles to track signal. Nothing to do for first two.
        spi_bus.receive(2);

        // Send channel. ADC Sends the first bit of IN0, which we don't care about.
        spi_bus.send(3, sensor.channel);

        //Wait out the rest of the IN0 reading being sent to us
        spi_bus.receive(11);

        // ADC is now tracking the channel we want
        spi_bus.receive(4);

        //Finally receive ADC value from the channel we care about
        let result = spi_bus.receive(12);
    }
    match adc_sensor {
        TemperatureADC => cs_pins.board_temperature_adc.set_high(),
        TetherADC => cs_pins.tether_measurement_adc.set_high(),
        MiscADC => cs_pins.misc_adc.set_high(),
    }
    result
}
