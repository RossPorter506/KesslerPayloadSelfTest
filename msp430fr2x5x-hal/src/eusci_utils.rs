// Things common to several eUSCI modes (e.g. UART, SPI, I2C)
// These are re-exported by the peripheral modes that use them.

/// Loopback settings
#[derive(Clone, Copy)]
pub enum Loopback {
    /// No loopback
    NoLoop,
    /// Tx feeds into Rx
    Loopback,
}

impl Loopback {
    #[inline(always)]
    pub(crate) fn to_bool(self) -> bool {
        match self {
            Loopback::NoLoop => false,
            Loopback::Loopback => true,
        }
    }
}

/// Bit order of transmit and receive
#[derive(Clone, Copy)]
pub enum BitOrder {
    /// LSB first (typically the default)
    LsbFirst,
    /// MSB first
    MsbFirst,
}

impl BitOrder {
    #[inline(always)]
    pub(crate) fn to_bool(self) -> bool {
        match self {
            BitOrder::LsbFirst => false,
            BitOrder::MsbFirst => true,
        }
    }
}

/// Number of bits per transaction
#[derive(Clone, Copy)]
pub enum BitCount {
    /// 8 bits
    EightBits,
    /// 7 bits
    SevenBits,
}

impl BitCount {
    #[inline(always)]
    pub(crate) fn to_bool(self) -> bool {
        match self {
            BitCount::EightBits => false,
            BitCount::SevenBits => true,
        }
    }
}