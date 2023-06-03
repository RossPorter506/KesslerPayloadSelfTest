use super::Steal;
use msp430fr2355 as pac;

pub enum Ucssel {
    Uclk,
    Aclk,
    Smclk,
}

/// Valid SPI clock sources.
pub enum UcsselSpi {
    /// Select ACLK for SPI
    Aclk=0b01,
    /// Use SMCLK for SPI
    Smclk=0b10,
}

pub struct UcxCtl0Uart {
    pub ucpen: bool,
    pub ucpar: bool,
    pub ucmsb: bool,
    pub uc7bit: bool,
    pub ucspb: bool,
    pub ucssel: Ucssel,
    pub ucrxeie: bool,
}
pub struct UcxCtl0Spi {
    pub ucckph: bool,
    pub ucckpl: bool,
    pub ucmsb: bool,
    pub uc7bit: bool,
    pub ucmst: bool,
    pub ucssel: UcsselSpi,
    //pub ucstem: bool, //only support 3-wire for now
}

pub trait EUsci: Steal {
    fn ctl0_reset(&self);

    // only call while in reset state
    fn brw_settings(&self, ucbr: u16);

    // only call while in reset state
    fn loopback(&self, loopback: bool);

    fn rx_rd(&self) -> u8;

    fn tx_wr(&self, val: u8);

    fn txie_set(&self);
    fn txie_clear(&self);
    fn rxie_set(&self);
    fn rxie_clear(&self);

    fn txifg_rd(&self) -> bool;
    fn rxifg_rd(&self) -> bool;

    fn iv_rd(&self) -> u16;
}

pub trait EUsciUart: EUsci {
    type Statw: UcaxStatwUart;

    // only call while in reset state
    fn ctl0_settings(&self, reg: UcxCtl0Uart);

    fn mctlw_settings(&self, ucos16: bool, ucbrs: u8, ucbrf: u8);

    fn statw_rd(&self) -> Self::Statw;
}

pub trait UcaxStatwUart {
    fn ucfe(&self) -> bool;
    fn ucoe(&self) -> bool;
    fn ucpe(&self) -> bool;
    fn ucbrk(&self) -> bool;
    fn ucbusy(&self) -> bool;
}

pub trait EUsciSpi: EUsci {
    type Statw: UcaxStatwSpi;

    // only call while in reset state
    fn ctl0_settings(&self, reg: UcxCtl0Spi);

    // only call while in reset state
    fn clear_mctlw(&self);

    fn statw_rd(&self) -> Self::Statw;
}
pub trait UcaxStatwSpi {
    fn ucfe(&self) -> bool;
    fn ucoe(&self) -> bool;
    fn ucbusy(&self) -> bool;
}

macro_rules! eusci_a_impl {
    ($EUsci:ident, $eusci:ident, $ucaxctlw0:ident, $ucaxctlw1:ident, $ucaxbrw:ident, $ucaxmctlw:ident,
     $ucaxstatw:ident, $ucaxrxbuf:ident, $ucaxtxbuf:ident, $ucaxie:ident, $ucaxifg:ident,
     $ucaxiv:ident, $Statw:ty) => {
        impl Steal for pac::$EUsci {
            #[inline(always)]
            unsafe fn steal() -> Self {
                pac::Peripherals::conjure().$EUsci
            }
        }

        impl EUsci for pac::$EUsci {
            #[inline(always)]
            fn ctl0_reset(&self) {
                self.$ucaxctlw0().write(|w| w.ucswrst().set_bit());
            }

            #[inline(always)]
            fn brw_settings(&self, ucbr: u16) {
                self.$ucaxbrw().write(|w| unsafe { w.bits(ucbr) });
            }

            #[inline(always)]
            fn loopback(&self, loopback: bool) {
                self.$ucaxstatw().write(|w| w.uclisten().bit(loopback));
            }

            #[inline(always)]
            fn rx_rd(&self) -> u8 {
                self.$ucaxrxbuf().read().ucrxbuf().bits()
            }

            #[inline(always)]
            fn tx_wr(&self, bits: u8) {
                self.$ucaxtxbuf()
                    .write(|w| unsafe { w.uctxbuf().bits(bits) });
            }

            #[inline(always)]
            fn txie_set(&self) {
                unsafe { self.$ucaxie().set_bits(|w| w.uctxie().set_bit()) };
            }

            #[inline(always)]
            fn txie_clear(&self) {
                unsafe { self.$ucaxie().clear_bits(|w| w.uctxie().clear_bit()) };
            }

            #[inline(always)]
            fn rxie_set(&self) {
                unsafe { self.$ucaxie().set_bits(|w| w.ucrxie().set_bit()) };
            }

            #[inline(always)]
            fn rxie_clear(&self) {
                unsafe { self.$ucaxie().clear_bits(|w| w.ucrxie().clear_bit()) };
            }

            #[inline(always)]
            fn txifg_rd(&self) -> bool {
                self.$ucaxifg().read().uctxifg().bit()
            }

            #[inline(always)]
            fn rxifg_rd(&self) -> bool {
                self.$ucaxifg().read().ucrxifg().bit()
            }

            #[inline(always)]
            fn iv_rd(&self) -> u16 {
                self.$ucaxiv().read().bits()
            }
        }
    };
}

eusci_a_impl!(
    E_USCI_A0,
    e_usci_a0,
    uca0ctlw0,
    uca0ctlw1,
    uca0brw,
    uca0mctlw,
    uca0statw,
    uca0rxbuf,
    uca0txbuf,
    uca0ie,
    uca0ifg,
    uca0iv,
    pac::e_usci_a0::uca0statw::R
);

eusci_a_impl!(
    E_USCI_A1,
    e_usci_a1,
    uca1ctlw0,
    uca1ctlw1,
    uca1brw,
    uca1mctlw,
    uca1statw,
    uca1rxbuf,
    uca1txbuf,
    uca1ie,
    uca1ifg,
    uca1iv,
    pac::e_usci_a1::uca1statw::R
);

macro_rules! eusci_a_impl_uart {
    ($EUsci:ident, $eusci:ident, $ucaxctlw0:ident, $ucaxctlw1:ident, $ucaxbrw:ident, $ucaxmctlw:ident,
     $ucaxstatw:ident, $ucaxrxbuf:ident, $ucaxtxbuf:ident, $ucaxie:ident, $ucaxifg:ident,
     $ucaxiv:ident, $Statw:ty) => {
        impl EUsciUart for pac::$EUsci {
            type Statw = $Statw;

            #[inline(always)]
            fn ctl0_settings(&self, reg: UcxCtl0Uart) {
                self.$ucaxctlw0().write(|w| {
                    w.ucpen()
                        .bit(reg.ucpen)
                        .ucpar()
                        .bit(reg.ucpar)
                        .ucmsb()
                        .bit(reg.ucmsb)
                        .uc7bit()
                        .bit(reg.uc7bit)
                        .ucspb()
                        .bit(reg.ucspb)
                        .ucssel()
                        .bits(reg.ucssel as u8)
                        .ucrxeie()
                        .bit(reg.ucrxeie)
                });
            }

            #[inline(always)]
            fn mctlw_settings(&self, ucos16: bool, ucbrs: u8, ucbrf: u8) {
                self.$ucaxmctlw.write(|w| unsafe {
                    w.ucos16()
                        .bit(ucos16)
                        .ucbrs()
                        .bits(ucbrs)
                        .ucbrf()
                        .bits(ucbrf)
                });
            }

            #[inline(always)]
            fn statw_rd(&self) -> Self::Statw {
                self.$ucaxstatw().read()
            }
        }

        impl UcaxStatwUart for $Statw {
            #[inline(always)]
            fn ucfe(&self) -> bool {
                self.ucfe().bit()
            }

            #[inline(always)]
            fn ucoe(&self) -> bool {
                self.ucoe().bit()
            }

            #[inline(always)]
            fn ucpe(&self) -> bool {
                self.ucpe().bit()
            }

            #[inline(always)]
            fn ucbrk(&self) -> bool {
                self.ucbrk().bit()
            }

            #[inline(always)]
            fn ucbusy(&self) -> bool {
                self.ucbusy().bit()
            }
        }
    };
}

eusci_a_impl_uart!(
    E_USCI_A0,
    e_usci_a0,
    uca0ctlw0,
    uca0ctlw1,
    uca0brw,
    uca0mctlw,
    uca0statw,
    uca0rxbuf,
    uca0txbuf,
    uca0ie,
    uca0ifg,
    uca0iv,
    pac::e_usci_a0::uca0statw::R
);

eusci_a_impl_uart!(
    E_USCI_A1,
    e_usci_a1,
    uca1ctlw0,
    uca1ctlw1,
    uca1brw,
    uca1mctlw,
    uca1statw,
    uca1rxbuf,
    uca1txbuf,
    uca1ie,
    uca1ifg,
    uca1iv,
    pac::e_usci_a1::uca1statw::R
);

macro_rules! eusci_a_impl_spi {
    ($EUsci:ident, $ucaxctlw0:ident, $ucaxctlw1:ident,
     $ucaxstatw:ident, $ucaxmctlw:ident, $Statw:ty) => {
        impl EUsciSpi for pac::$EUsci {
            type Statw = $Statw;

            #[inline(always)]
            fn ctl0_settings(&self, reg: UcxCtl0Spi) {
                self.$ucaxctlw0().write(|w| {
                    w.ucckph()
                        .bit(reg.ucckph)
                        .ucckpl()
                        .bit(reg.ucckpl)
                        .ucmsb()
                        .bit(reg.ucmsb)
                        .uc7bit()
                        .bit(reg.uc7bit)
                        .ucmst()
                        .bit(reg.ucmst)
                        .ucssel()
                        .bits(reg.ucssel as u8)
                });
            }

            #[inline(always)]
            fn clear_mctlw(&self) {
                self.$ucaxmctlw.write(|w| unsafe {
                    w.ucos16()
                        .bit(false)
                        .ucbrs()
                        .bits(0)
                        .ucbrf()
                        .bits(0)
                });
            }
            
            #[inline(always)]
            fn statw_rd(&self) -> Self::Statw {
                self.$ucaxstatw().read()
            }
        }

        impl UcaxStatwSpi for $Statw {
            #[inline(always)]
            fn ucfe(&self) -> bool {
                self.ucfe().bit()
            }

            #[inline(always)]
            fn ucoe(&self) -> bool {
                self.ucoe().bit()
            }

            //This bit is not exposed in the PAC crate
            #[inline(always)]
            fn ucbusy(&self) -> bool {
                //self.ucbusy().bit()
            }
        }
    };
}

eusci_a_impl_spi!(
    E_USCI_A0,
    uca0ctlw0_spi,
    uca0ctlw1,
    uca0statw_spi,
    uca0mctlw,
    pac::e_usci_a0::uca0statw_spi::R
);

eusci_a_impl_spi!(
    E_USCI_A1,
    uca1ctlw0_spi,
    uca1ctlw1,
    uca1statw_spi,
    uca1mctlw,
    pac::e_usci_a1::uca1statw_spi::R
);

macro_rules! eusci_b_impl {
    ($EUsci:ident, $eusci:ident, $ucbxctlw0:ident, $ucbxctlw1:ident, $ucbxbrw:ident,
     $ucbxstatw:ident, $ucbxrxbuf:ident, $ucbxtxbuf:ident, $ucbxie:ident, $ucbxifg:ident,
     $ucbxiv:ident, $Statw:ty) => {
        impl Steal for pac::$EUsci {
            #[inline(always)]
            unsafe fn steal() -> Self {
                pac::Peripherals::conjure().$EUsci
            }
        }

        impl EUsci for pac::$EUsci {
            #[inline(always)]
            fn ctl0_reset(&self) {
                self.$ucbxctlw0().write(|w| w.ucswrst().set_bit());
            }

            #[inline(always)]
            fn brw_settings(&self, ucbr: u16) {
                self.$ucbxbrw().write(|w| unsafe { w.bits(ucbr) });
            }

            #[inline(always)]
            fn loopback(&self, loopback: bool) {
                self.$ucbxstatw().write(|w| w.uclisten().bit(loopback));
            }

            #[inline(always)]
            fn rx_rd(&self) -> u8 {
                self.$ucbxrxbuf().read().ucrxbuf().bits()
            }

            #[inline(always)]
            fn tx_wr(&self, bits: u8) {
                self.$ucbxtxbuf()
                    .write(|w| unsafe { w.uctxbuf().bits(bits) });
            }

            #[inline(always)]
            fn txie_set(&self) {
                unsafe { self.$ucbxie().set_bits(|w| w.uctxie().set_bit()) };
            }

            #[inline(always)]
            fn txie_clear(&self) {
                unsafe { self.$ucbxie().clear_bits(|w| w.uctxie().clear_bit()) };
            }

            #[inline(always)]
            fn rxie_set(&self) {
                unsafe { self.$ucbxie().set_bits(|w| w.ucrxie().set_bit()) };
            }

            #[inline(always)]
            fn rxie_clear(&self) {
                unsafe { self.$ucbxie().clear_bits(|w| w.ucrxie().clear_bit()) };
            }

            #[inline(always)]
            fn txifg_rd(&self) -> bool {
                self.$ucbxifg().read().uctxifg().bit()
            }

            #[inline(always)]
            fn rxifg_rd(&self) -> bool {
                self.$ucbxifg().read().ucrxifg().bit()
            }

            #[inline(always)]
            fn iv_rd(&self) -> u16 {
                self.$ucbxiv().read().bits()
            }
        }
    };
}

eusci_b_impl!(
    E_USCI_B0,
    e_usci_b0,
    ucb0ctlw0_spi,
    ucb0ctlw1,
    ucb0brw,
    ucb0statw_spi,
    ucb0rxbuf,
    ucb0txbuf,
    ucb0ie_spi,
    ucb0ifg_spi,
    ucb0iv,
    pac::e_usci_b0::ucb0statw_spi::R
);

eusci_b_impl!(
    E_USCI_B1,
    e_usci_b1,
    ucb1ctlw0_spi,
    ucb1ctlw1,
    ucb1brw,
    ucb1statw_spi,
    ucb1rxbuf,
    ucb1txbuf,
    ucb1ie_spi,
    ucb1ifg_spi,
    ucb1iv,
    pac::e_usci_b1::ucb1statw_spi::R
);

macro_rules! eusci_b_impl_spi {
    ($EUsci:ident, $eusci:ident, $ucbxctlw0:ident, $ucbxctlw1:ident,
     $ucbxstatw:ident, $Statw:ty) => {
        impl EUsciSpi for pac::$EUsci {
            type Statw = $Statw;

            #[inline(always)]
            fn ctl0_settings(&self, reg: UcxCtl0Spi) {
                self.$ucbxctlw0().write(|w| {
                    w.ucckph()
                        .bit(reg.ucckph)
                        .ucckpl()
                        .bit(reg.ucckpl)
                        .ucmsb()
                        .bit(reg.ucmsb)
                        .uc7bit()
                        .bit(reg.uc7bit)
                        .ucmst()
                        .bit(reg.ucmst)
                        .ucssel()
                        .bits(reg.ucssel as u8)
                });
            }

            // The modulation control only needs to be cleared on 
            // peripherals that share with UART on eUSCI_A, so nothing to do here.
            #[inline(always)]
            fn clear_mctlw(&self) {return;}

            #[inline(always)]
            fn statw_rd(&self) -> Self::Statw {
                self.$ucbxstatw().read()
            }
        }

        impl UcaxStatwSpi for $Statw {
            #[inline(always)]
            fn ucfe(&self) -> bool {
                self.ucfe().bit()
            }

            #[inline(always)]
            fn ucoe(&self) -> bool {
                self.ucoe().bit()
            }

            //This bit is not exposed in the PAC crate
            #[inline(always)]
            fn ucbusy(&self) -> bool {
                todo!() //self.ucbusy().bit()
            }
        }
    };
}

eusci_b_impl_spi!(
    E_USCI_B0,
    e_usci_b0,
    ucb0ctlw0_spi,
    ucb0ctlw1,
    ucb0statw_spi,
    pac::e_usci_b0::ucb0statw_spi::R
);

eusci_b_impl_spi!(
    E_USCI_B1,
    e_usci_b1,
    ucb1ctlw0_spi,
    ucb1ctlw1,
    ucb1statw_spi,
    pac::e_usci_b1::ucb1statw_spi::R
);