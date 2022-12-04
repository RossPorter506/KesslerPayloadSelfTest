use embedded_hal::serial::Write;
use msp430fr2x5x_hal::serial::{SerialUsci, Tx};
use ufmt::uWrite;
use void::Void;

struct SerialWriter<USCI: SerialUsci>{
    serial: Tx<USCI>
}
impl<USCI: SerialUsci> SerialWriter<USCI>{
    pub fn new(serial: Tx<USCI>) -> SerialWriter<USCI> {
        SerialWriter{serial}
    }
}
impl<USCI: SerialUsci> uWrite for SerialWriter<USCI>{
    type Error = nb::Error<Void>;
    fn write_char(&mut self, c: char) -> Result<(), Self::Error>{
        while self.serial.write(c as u8).is_err(){}
        Ok(())
    }

    fn write_str(&mut self, string: &str) -> Result<(), Self::Error> {
        for chr in string.chars() {
            self.write_char(chr).ok();
        }
        Ok(())
    }
}
