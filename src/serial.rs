use embedded_hal::serial::{Write, Read};
use msp430fr2x5x_hal::serial::{SerialUsci, Tx, Rx};
use ufmt::uWrite;
use void::Void;

pub struct SerialWriter<USCI: SerialUsci>{
    serial: Tx<USCI>
}
impl<USCI: SerialUsci> SerialWriter<USCI>{
    pub fn new(serial: Tx<USCI>) -> SerialWriter<USCI> {
        SerialWriter{serial}
    }
    pub fn return_pin(self) -> Tx<USCI> {
        self.serial
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

// Block until we receive any packet over serial
pub fn wait_for_any_packet<USCI: SerialUsci>(serial_reader: &mut Rx<USCI>) -> u8{
    loop {
        match serial_reader.read() {
            Ok(packet) => return packet,
            _ => (),
        }
    }
}
// Block until we receive the specified character
pub fn wait_for_character<USCI: SerialUsci>(wanted_char: u8, serial_reader: &mut Rx<USCI>) {
    while wait_for_any_packet(serial_reader) != wanted_char {}
}
pub fn wait_for_string<USCI: SerialUsci>(wanted_str: &str, serial_reader: &mut Rx<USCI>) {
    for chr in wanted_str.as_bytes(){
        wait_for_character(*chr, serial_reader);
    }
}