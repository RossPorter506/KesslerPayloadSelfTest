use core::cell::{RefCell, UnsafeCell};

use critical_section::Mutex;
use embedded_hal::serial::{Write, Read};
use msp430fr2355::E_USCI_A1;
use msp430fr2x5x_hal::serial::{SerialUsci, Tx, Rx};
use ufmt::{uWrite, uwrite, uwriteln, uDisplay};
use void::Void;

//Macros to only print if debug_print feature is enabled
#[allow(unused_macros)]
macro_rules! dbg_uwriteln {
    ($first:tt $(, $( $rest:tt )* )?) => {    
        #[cfg(feature = "debug_print")]
        {uwrite!($first, "[....] ").ok(); uwriteln!($first, $( $($rest)* )*).ok();}
    }
}

#[allow(unused_macros)]
macro_rules! dbg_uwrite {
    ($first:tt $(, $( $rest:tt )* )?) => {    
        #[cfg(feature = "debug_print")]
        {uwrite!($first, "[....] ").ok(); uwrite!($first, $( $($rest)* )*).ok();}
    }
}

// Colour printing
macro_rules! uwrite_coloured {    
    ($a:expr, $b:expr, $c:expr) => {
        match $c{
            $crate::serial::TextColours::Red => uwrite!($a, "\x1b[31m{}\x1b[0m", $b).ok(),
            $crate::serial::TextColours::Green => uwrite!($a, "\x1b[32m{}\x1b[0m", $b).ok(),  
            $crate::serial::TextColours::Yellow => uwrite!($a, "\x1b[33m{}\x1b[0m", $b).ok(),
        }
    }
}

pub static SERIAL_WR: Mutex<UnsafeCell<Option< SerialWriter<E_USCI_A1> >>> = Mutex::new(UnsafeCell::new(None));

#[macro_export]
macro_rules! println {
    ($( $rest:tt )*) => {    
        critical_section::with(|cs| {
            if let Some(serial) = unsafe{&mut *$crate::serial::SERIAL_WR.borrow(cs).get()}.as_mut() {
                uwriteln!(serial,  $($rest)*).ok();
            }
        })
    }
}

#[macro_export]
macro_rules! print {
    ($( $rest:tt )*) => {    
        critical_section::with(|cs| {
            if let Some(serial) = unsafe{&mut *$crate::serial::SERIAL_WR.borrow(cs).get()}.as_mut() {
                uwrite!(serial,  $($rest)*).ok();
            }
        })
    }
}

#[macro_export]
macro_rules! dbg_println {
    ($( $rest:tt )*) => {    
        #[cfg(feature = "debug_print")]
        {
            $crate::print!("[....] "); $crate::println!($($rest)*)
        }
    }
}

#[macro_export]
macro_rules! dbg_print {
    ($( $rest:tt )*) => {    
        #[cfg(feature = "debug_print")]
        {
            $crate::print!("[....] "); $crate::print!($($rest)*)
        }
    }
}

pub(crate) use uwrite_coloured;


pub enum TextColours {
    Red, 
    Green, 
    Yellow,
}

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

/*  Fixed point numbers from the 'fixed' library do not implement uDisplay from the 'ufmt' library
    We can't implement an external trait on an external struct.
    Instead, we make a trait Printable which can be implemented on fixed numbers by calling x.to_prnt()
    This trait returns a newtype PrintableFixedI64 which can implement uDisplay, since it's defined inside this project
*/
pub struct PrintableFixedI64<const N: i32>(fixed::FixedI64::<N>);
impl<const N: i32> uDisplay for PrintableFixedI64<N> {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where W: uWrite + ?Sized {
        let frac_bits = fixed::FixedI64::<N>::FRAC_BITS;
        let frac_mask: u64 = (1 << frac_bits) - 1;

        let mut fxd = self.0;
            
        let sign = if fxd < 0 {'-'} else {'+'};

        // Fractional bits are always positive, even for negative numbers. Make the number positive so they make sense
        if sign == '-' {fxd *= -1;} 

        let int: i32 = fxd.to_num();
        uwrite!(f, "{}{}", sign, int).ok();

        let mut frac: u64 = (fxd.frac().to_bits() as u64) & frac_mask;

        if frac != 0 { uwrite!(f, ".").ok(); }

        let mut precision = 0;
        while frac != 0 && precision < 10 {
            frac *= 10;
            let digit = frac >> frac_bits;
            uwrite!(f, "{}", digit).ok();
            frac &= frac_mask;
            precision += 1;
        }
        Ok(())
    }
}

pub trait Printable<const N: i32> {
    fn printable(&self) -> PrintableFixedI64<N>;
}
impl<const N: i32> Printable<N> for fixed::FixedI64::<N> {
    fn printable(&self) -> PrintableFixedI64<N> {
        PrintableFixedI64::<N>(*self)
    }
}

// Block until we receive any packet over serial
pub fn wait_for_any_packet<USCI: SerialUsci>(serial_reader: &mut Rx<USCI>) -> u8{
    loop {
        if let Ok(packet) = serial_reader.read(){
            return packet;
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
pub fn read_num<USCI: SerialUsci>(serial_reader: &mut Rx<USCI> ) -> i32 {
    loop {
        match maybe_read_num(serial_reader) {
            Some(n) => return n,
            _ => println!("Error parsing number. Try again: "),
        };
    }
}

fn is_ascii_number(c: u8) -> bool {
    (ASCII_ZERO..=ASCII_NINE).contains(&c)
}

const ASCII_ZERO: u8 = b'0';
const ASCII_NINE: u8 = b'9';
const CARRIAGE_RETURN: u8 = b'\r'; 
const NEGATIVE_SIGN: u8 = b'-';