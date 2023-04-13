use embedded_hal::serial::{Write, Read};
use msp430fr2x5x_hal::serial::{SerialUsci, Tx, Rx};
use ufmt::{uWrite, uwrite, uwriteln};
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

// Query the user for a number. Return None if invalid.
pub fn maybe_read_num<USCI: SerialUsci>(serial_reader: &mut Rx<USCI>) -> Option<i32> {
    let mut num: i32 = 0;
    let mut sign = 1;
    // First character needs to be treated differently since '-' makes a number negative when first, but is invalid in other places.
    match wait_for_any_packet(serial_reader) {
        CARRIAGE_RETURN => return None,
        NEGATIVE_SIGN => sign = -1, // Make number negative afterwards
        n if is_ascii_number(n) => {num = (n - ASCII_ZERO) as i32},
        _ => return None,
    }
    loop{
        match wait_for_any_packet(serial_reader) {
            CARRIAGE_RETURN => break,
            n if is_ascii_number(n) => {num = num * 10 + (n - ASCII_ZERO) as i32},
            _ => return None,
        }
    }

    Some(sign*num)
}

// Repeatedly queries the user to input a number until a valid one is received.
pub fn read_num<USCI: SerialUsci>(debug_writer: &mut SerialWriter<USCI>, serial_reader: &mut Rx<USCI> ) -> i32 {
    loop {
        match maybe_read_num(serial_reader) {
            Some(n) => return n,
            _ => uwrite!(debug_writer, "Error parsing number. Try again: ").ok(),
        };
    }
}

fn is_ascii_number(c: u8) -> bool {
    c >= ASCII_ZERO && c <= ASCII_NINE
}

const ASCII_ZERO: u8 = '0' as u8;
const ASCII_NINE: u8 = '9' as u8;
const CARRIAGE_RETURN: u8 = '\r' as u8; 
const NEGATIVE_SIGN: u8 = '-' as u8;