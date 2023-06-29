//! SPI
//!
//! The peripherals E_USCI_A0, E_USCI_A1, E_USCI_B0, and E_USCI_B1 can be configured as an SPI bus.
//! After configuring the E_USCI peripheral, the SPI bus can be obtained by
//! converting the appropriate GPIO pins to the alternate function corresponding to SPI.

use core::marker::ConstParamTy;
use embedded_hal::spi::{FullDuplex};
use msp430fr2355 as pac;
use crate::clock::{Aclk, Smclk, Clock};
use crate::gpio::{Alternate1, Pin, Pin1, Pin2, Pin3, Pin5, Pin6, Pin7, P1, P4, Output};
use crate::hw_traits::eusci::{EUsciSpi, UcxCtl0Spi, UcsselSpi, UcaxStatwSpi};
//Re-export so users can do spi::BitOrder, etc.
pub use crate::eusci_utils::{BitOrder, BitCount, Loopback};
pub use crate::hw_traits::eusci::UcsselSpi as SpiClock;

/// Spi bus object. Use to send or receive data.
/// 
/// Note that read() merely reads the receive buffer, so a packet must be first sent to a slave before data can be read. 
pub struct SpiBus<USCI: SpiUsci>{
    sck:  USCI::SckPin,
    mosi: USCI::MosiPin,
    miso: USCI::MisoPin,
    /// Public for debugging only
    pub usci: USCI, // TODO: Make private after debugging
}

impl<USCI: SpiUsci> FullDuplex<u8> for SpiBus<USCI> {
    type Error = SpiError;
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let usci = unsafe { USCI::steal() };
        if usci.rxifg_rd() {
            let status_reg = usci.statw_rd();
            let word = usci.rx_rd();

            if status_reg.ucfe() {
                Err(nb::Error::Other(SpiError::Framing))
            } else if status_reg.ucoe() {
                Err(nb::Error::Other(SpiError::Overrun(word)))
            } else {
                Ok(word)
            }
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
    fn send(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        let usci = unsafe { USCI::steal() };
        if usci.txifg_rd() {
            usci.tx_wr(word);
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

// Blocking implementations
impl<USCI: SpiUsci> embedded_hal::blocking::spi::transfer::Default<u8> for SpiBus<USCI> {}
impl<USCI: SpiUsci> embedded_hal::blocking::spi::write::Default<u8> for SpiBus<USCI> {}

impl<USCI:SpiUsci> SpiBus<USCI> {
    /// Reconfigure common elements without remaking the bus.
    #[inline]
    pub fn reconfigure(&mut self,
        polarity: Polarity, phase: Phase, 
        order: BitOrder, count: BitCount,
        clock: UcsselSpi,
        ) {
        
        self.usci.ctl0_reset();
        self.usci.ctl0_settings(UcxCtl0Spi {
            ucckph: phase.into(), ucckpl: polarity.into(), 
            uc7bit: count.to_bool(), ucmsb: order.to_bool(), 
            ucmst: true, ucssel: clock });
    }

    /// Check if transmission has completed. Check this before de-asserting chip select.
    #[inline]
    pub fn flush(&mut self) -> nb::Result<(), SpiError> {
        let usci = unsafe { USCI::steal() };
        let status_reg = usci.statw_rd();
        if status_reg.ucbusy() {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }

    /// Consume the SPI bus to return the GPIO pins.
    #[inline]
    pub fn return_pins(self) -> (USCI::SckPin, USCI::MosiPin, USCI::MisoPin) {
        (self.sck, self.mosi, self.miso)
    }
}

/// SPI Error conditions
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SpiError {
    /// An Overrun occurred: Unread received data was overwritten with newer data. Contains the latest word (which is still valid).
    Overrun(u8),
    /// An SPI framing error
    Framing,
}

/// SPI Configuration object. Will produce an SPIBus object when fully configured.
pub struct SpiConfig<USCI: SpiUsci, State>{
    usci: USCI,
    phase: Phase,
    polarity: Polarity,
    order: BitOrder,
    count: BitCount,
    loopback: Loopback,
    state: State,
}

macro_rules! update_spi_config {
    ($conf:expr, $state:expr) => {
        SpiConfig {
            usci: $conf.usci,
            polarity: $conf.polarity,
            phase: $conf.phase,
            order: $conf.order,
            count: $conf.count,
            loopback: $conf.loopback,
            state: $state,
        }
    };
}

impl<USCI: SpiUsci> SpiConfig<USCI, NoClockSet> {
    /// Begin assembling an SPI configuration. 
    pub fn new(
        usci: USCI,
        polarity: Polarity,
        phase: Phase,
        order: BitOrder,
        count: BitCount,
        loopback: Loopback,
        baudrate: u32) -> Self {
        
        return SpiConfig {
            usci, polarity, phase, order, count, loopback, state: NoClockSet { baudrate }
        }
    }
    /// Configure SPI to use ACLK.
    #[inline(always)]
    pub fn use_aclk(self, aclk: &Aclk) -> SpiConfig<USCI, ClockSet> {
        update_spi_config!(
            self,
            ClockSet {
                prescaler: calculate_prescaler(aclk.freq() as u32, self.state.baudrate),
                clksel: UcsselSpi::Aclk,
            }
        )
    }

    /// Configure SPI to use SMCLK.
    #[inline(always)]
    pub fn use_smclk(self, smclk: &Smclk) -> SpiConfig<USCI, ClockSet> {
        update_spi_config!(
            self,
            ClockSet {
                prescaler: calculate_prescaler(smclk.freq(), self.state.baudrate),
                clksel: UcsselSpi::Smclk,
            }
        )
    }
}
impl<USCI: SpiUsci> SpiConfig<USCI, ClockSet> {
    #[inline]
    fn config_hw(self) -> USCI{
        let ClockSet {
            prescaler,
            clksel,
        } = self.state;
        let usci = self.usci;

        usci.ctl0_reset();
        usci.clear_mctlw();
        usci.brw_settings(prescaler);
        usci.loopback(self.loopback.to_bool());
        usci.ctl0_settings(UcxCtl0Spi {
            ucckph: self.phase.into(),
            ucckpl: self.polarity.into(),
            ucmsb: self.order.to_bool(),
            uc7bit: self.count.to_bool(),
            ucmst: true, // only support master mode for now
            ucssel: clksel,
        });
        usci
    }
    /// Consume SPI pins, apply the configuration and reset the SPI peripheral.
    pub fn apply_config<SCK: Into<USCI::SckPin>, MOSI: Into<USCI::MosiPin>, MISO: Into<USCI::MisoPin>>
            (self, sck_pin: SCK, mosi_pin: MOSI, miso_pin: MISO) -> SpiBus<USCI> {
        let usci = self.config_hw();
        SpiBus::<USCI>{
            sck: sck_pin.into(), 
            mosi: mosi_pin.into(), 
            miso: miso_pin.into(), 
            usci: usci,
        }
    }
}

fn calculate_prescaler(clk_freq: u32, baudrate: u32) -> u16 {
    let prescaler = clk_freq / baudrate.max(1);
    return prescaler.min(u16::MAX as u32) as u16;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, ConstParamTy)]
/// SPI Polarity
pub enum Polarity {
    /// SCK idles at Vcc
    IdleHigh=1,
    /// SCK idles at GND
    IdleLow=0
}
impl From<Polarity> for bool {
    fn from(value: Polarity) -> bool {
        match value {
            Polarity::IdleHigh => true,
            Polarity::IdleLow => false,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug, ConstParamTy)]
/// SPI Phase.
pub enum Phase {
    /// Data is captured on the first UCLK edge and changed on the following edge
    CaptureOnFirstEdge=1,
    /// Data is changed on the first UCLK edge and captured on the following edge
    CaptureOnSecondEdge=0,
}
impl From<Phase> for bool {
    fn from(value: Phase) -> bool {
        match value {
            Phase::CaptureOnFirstEdge => true,
            Phase::CaptureOnSecondEdge => false,
        }
    }
}

/// Typestate for an SpiConfig that has not chosen a clock yet.
pub struct NoClockSet {
    baudrate: u32,
}
/// Typestate for an SpiConfig that has chosen a clock.
pub struct ClockSet {
    prescaler: u16,
    clksel: UcsselSpi,
}

/// Shared trait that all SPI-capable peripherals implement. See macro calls below for pin numbers.
pub trait SpiUsci: EUsciSpi {
    /// Pin used for serial SCK
    type SckPin;
    /// Pin used for MOSI
    type MosiPin;
    /// Pin used for MISO
    type MisoPin;
}

macro_rules! impl_SpiUsci {
    ($EUsci:ident, 
     $ClkPinType:ty, 
     $MosiPinType:ty, 
     $MisoPinType:ty
     ) => {
        impl SpiUsci for pac::$EUsci {
            type SckPin = $ClkPinType;
            type MosiPin = $MosiPinType;
            type MisoPin = $MisoPinType;
        }
    };
}

impl_SpiUsci!(E_USCI_A0, 
    Pin<P1, Pin5, Alternate1<Output>>, // Sck
    Pin<P1, Pin6, Alternate1<Output>>, // Mosi
    Pin<P1, Pin7, Alternate1<Output>>);// Miso

impl_SpiUsci!(E_USCI_A1, 
    Pin<P4, Pin1, Alternate1<Output>>,
    Pin<P4, Pin3, Alternate1<Output>>,
    Pin<P4, Pin2, Alternate1<Output>>);

impl_SpiUsci!(E_USCI_B0, 
    Pin<P1, Pin1, Alternate1<Output>>,
    Pin<P1, Pin2, Alternate1<Output>>, 
    Pin<P1, Pin3, Alternate1<Output>>);

impl_SpiUsci!(E_USCI_B1, 
    Pin<P4, Pin5, Alternate1<Output>>,
    Pin<P4, Pin6, Alternate1<Output>>,
    Pin<P4, Pin7, Alternate1<Output>>);
