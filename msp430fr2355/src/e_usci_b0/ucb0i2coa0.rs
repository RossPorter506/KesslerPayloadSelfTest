#[doc = "Register `UCB0I2COA0` reader"]
pub struct R(crate::R<UCB0I2COA0_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<UCB0I2COA0_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<UCB0I2COA0_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<UCB0I2COA0_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `UCB0I2COA0` writer"]
pub struct W(crate::W<UCB0I2COA0_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<UCB0I2COA0_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl core::ops::DerefMut for W {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<crate::W<UCB0I2COA0_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<UCB0I2COA0_SPEC>) -> Self {
        W(writer)
    }
}
#[doc = "Field `I2COA0` reader - I2C own address"]
pub type I2COA0_R = crate::FieldReader<u16, u16>;
#[doc = "Field `I2COA0` writer - I2C own address"]
pub type I2COA0_W<'a, const O: u8> = crate::FieldWriter<'a, u16, UCB0I2COA0_SPEC, u16, u16, 10, O>;
#[doc = "Field `UCOAEN` reader - Own Address enable register"]
pub type UCOAEN_R = crate::BitReader<UCOAEN_A>;
#[doc = "Own Address enable register\n\nValue on reset: 0"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UCOAEN_A {
    #[doc = "0: The slave address defined in I2COA0 is disabled"]
    DISABLE = 0,
    #[doc = "1: The slave address defined in I2COA0 is enabled"]
    ENABLE = 1,
}
impl From<UCOAEN_A> for bool {
    #[inline(always)]
    fn from(variant: UCOAEN_A) -> Self {
        variant as u8 != 0
    }
}
impl UCOAEN_R {
    #[doc = "Get enumerated values variant"]
    #[inline(always)]
    pub fn variant(&self) -> UCOAEN_A {
        match self.bits {
            false => UCOAEN_A::DISABLE,
            true => UCOAEN_A::ENABLE,
        }
    }
    #[doc = "Checks if the value of the field is `DISABLE`"]
    #[inline(always)]
    pub fn is_disable(&self) -> bool {
        *self == UCOAEN_A::DISABLE
    }
    #[doc = "Checks if the value of the field is `ENABLE`"]
    #[inline(always)]
    pub fn is_enable(&self) -> bool {
        *self == UCOAEN_A::ENABLE
    }
}
#[doc = "Field `UCOAEN` writer - Own Address enable register"]
pub type UCOAEN_W<'a, const O: u8> = crate::BitWriter<'a, u16, UCB0I2COA0_SPEC, UCOAEN_A, O>;
impl<'a, const O: u8> UCOAEN_W<'a, O> {
    #[doc = "The slave address defined in I2COA0 is disabled"]
    #[inline(always)]
    pub fn disable(self) -> &'a mut W {
        self.variant(UCOAEN_A::DISABLE)
    }
    #[doc = "The slave address defined in I2COA0 is enabled"]
    #[inline(always)]
    pub fn enable(self) -> &'a mut W {
        self.variant(UCOAEN_A::ENABLE)
    }
}
#[doc = "Field `UCGCEN` reader - General call response enable"]
pub type UCGCEN_R = crate::BitReader<UCGCEN_A>;
#[doc = "General call response enable\n\nValue on reset: 0"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UCGCEN_A {
    #[doc = "0: Do not respond to a general call"]
    UCGCEN_0 = 0,
    #[doc = "1: Respond to a general call"]
    UCGCEN_1 = 1,
}
impl From<UCGCEN_A> for bool {
    #[inline(always)]
    fn from(variant: UCGCEN_A) -> Self {
        variant as u8 != 0
    }
}
impl UCGCEN_R {
    #[doc = "Get enumerated values variant"]
    #[inline(always)]
    pub fn variant(&self) -> UCGCEN_A {
        match self.bits {
            false => UCGCEN_A::UCGCEN_0,
            true => UCGCEN_A::UCGCEN_1,
        }
    }
    #[doc = "Checks if the value of the field is `UCGCEN_0`"]
    #[inline(always)]
    pub fn is_ucgcen_0(&self) -> bool {
        *self == UCGCEN_A::UCGCEN_0
    }
    #[doc = "Checks if the value of the field is `UCGCEN_1`"]
    #[inline(always)]
    pub fn is_ucgcen_1(&self) -> bool {
        *self == UCGCEN_A::UCGCEN_1
    }
}
#[doc = "Field `UCGCEN` writer - General call response enable"]
pub type UCGCEN_W<'a, const O: u8> = crate::BitWriter<'a, u16, UCB0I2COA0_SPEC, UCGCEN_A, O>;
impl<'a, const O: u8> UCGCEN_W<'a, O> {
    #[doc = "Do not respond to a general call"]
    #[inline(always)]
    pub fn ucgcen_0(self) -> &'a mut W {
        self.variant(UCGCEN_A::UCGCEN_0)
    }
    #[doc = "Respond to a general call"]
    #[inline(always)]
    pub fn ucgcen_1(self) -> &'a mut W {
        self.variant(UCGCEN_A::UCGCEN_1)
    }
}
impl R {
    #[doc = "Bits 0:9 - I2C own address"]
    #[inline(always)]
    pub fn i2coa0(&self) -> I2COA0_R {
        I2COA0_R::new((self.bits & 0x03ff) as u16)
    }
    #[doc = "Bit 10 - Own Address enable register"]
    #[inline(always)]
    pub fn ucoaen(&self) -> UCOAEN_R {
        UCOAEN_R::new(((self.bits >> 10) & 1) != 0)
    }
    #[doc = "Bit 15 - General call response enable"]
    #[inline(always)]
    pub fn ucgcen(&self) -> UCGCEN_R {
        UCGCEN_R::new(((self.bits >> 15) & 1) != 0)
    }
}
impl W {
    #[doc = "Bits 0:9 - I2C own address"]
    #[inline(always)]
    pub fn i2coa0(&mut self) -> I2COA0_W<0> {
        I2COA0_W::new(self)
    }
    #[doc = "Bit 10 - Own Address enable register"]
    #[inline(always)]
    pub fn ucoaen(&mut self) -> UCOAEN_W<10> {
        UCOAEN_W::new(self)
    }
    #[doc = "Bit 15 - General call response enable"]
    #[inline(always)]
    pub fn ucgcen(&mut self) -> UCGCEN_W<15> {
        UCGCEN_W::new(self)
    }
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u16) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "eUSCI_Bx I2C Own Address 0 Register\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`reset`](crate::generic::Reg::reset), [`write`](crate::generic::Reg::write), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [ucb0i2coa0](index.html) module"]
pub struct UCB0I2COA0_SPEC;
impl crate::RegisterSpec for UCB0I2COA0_SPEC {
    type Ux = u16;
}
#[doc = "`read()` method returns [ucb0i2coa0::R](R) reader structure"]
impl crate::Readable for UCB0I2COA0_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [ucb0i2coa0::W](W) writer structure"]
impl crate::Writable for UCB0I2COA0_SPEC {
    type Writer = W;
}
#[doc = "`reset()` method sets UCB0I2COA0 to value 0"]
impl crate::Resettable for UCB0I2COA0_SPEC {
    #[inline(always)]
    fn reset_value() -> Self::Ux {
        0
    }
}
