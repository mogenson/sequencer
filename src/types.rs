use std::convert::TryFrom;
use std::io::ErrorKind;

#[allow(non_camel_case_types)]
#[derive(Default, Clone, Copy, Debug)]
pub struct u2(u8);

impl u2 {
    pub const MAX: u8 = 0x3;
    pub const ZERO: Self = Self(0);
}

impl TryFrom<u8> for u2 {
    type Error = ErrorKind;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl TryFrom<usize> for u2 {
    type Error = ErrorKind;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value <= Self::MAX.into() {
            Ok(Self(value as u8))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl From<u2> for u8 {
    fn from(value: u2) -> Self {
        value.0
    }
}

impl From<u2> for usize {
    fn from(value: u2) -> Self {
        value.0 as usize
    }
}

#[allow(non_camel_case_types)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct u4(u8);

impl u4 {
    pub const MAX: u8 = 0xF;
    pub const ZERO: Self = Self(0);

    pub fn wrapping_add(self, rhs: Self) -> Self {
        let mut x = self.0.wrapping_add(rhs.0);
        if x > Self::MAX {
            x -= Self::MAX;
        }
        Self(x)
    }
}

impl TryFrom<u8> for u4 {
    type Error = ErrorKind;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl TryFrom<u32> for u4 {
    type Error = ErrorKind;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value <= Self::MAX.into() {
            Ok(Self(value as u8))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl TryFrom<usize> for u4 {
    type Error = ErrorKind;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value <= Self::MAX.into() {
            Ok(Self(value as u8))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl From<u4> for u8 {
    fn from(value: u4) -> Self {
        value.0
    }
}

impl From<u4> for usize {
    fn from(value: u4) -> Self {
        value.0 as usize
    }
}

#[allow(non_camel_case_types)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct u7(u8);

impl u7 {
    pub const MAX: u8 = 0x7F;
    pub const ZERO: Self = Self(0);
}

impl TryFrom<u8> for u7 {
    type Error = ErrorKind;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl TryFrom<usize> for u7 {
    type Error = ErrorKind;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value <= Self::MAX.into() {
            Ok(Self(value as u8))
        } else {
            Err(ErrorKind::InvalidInput)
        }
    }
}

impl From<u7> for u8 {
    fn from(value: u7) -> Self {
        value.0
    }
}

impl From<u7> for usize {
    fn from(value: u7) -> Self {
        value.0 as usize
    }
}

#[derive(Debug)]
pub struct Note {
    pub pitch: u7,
    pub velocity: u7,
    pub duration: u4,
}

impl Note {
    pub fn from_pitch(pitch: u7) -> Self {
        Self {
            pitch,
            velocity: u7::ZERO,
            duration: u4::ZERO,
        }
    }
}

#[derive(Debug)]
pub struct Param {
    pub controller: Controller,
    pub value: u7,
}

impl Param {
    pub fn from_controller(controller: Controller) -> Self {
        Self {
            controller,
            value: u7::ZERO,
        }
    }
}

// output from sequencer
pub enum Event {
    NoteOn {
        channel: u4,
        pitch: u7,
        velocity: u7,
    },
    NoteOff {
        channel: u4,
        pitch: u7,
    },
    ControllerChange {
        channel: u4,
        controller: u7,
        value: u7,
    },
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Controller {
    Modulation,
    Breath,
    Volume,
    Pan,
}

impl Controller {
    pub fn number(&self) -> u7 {
        match *self {
            Self::Modulation => u7::try_from(1 as u8).unwrap(),
            Self::Breath => u7::try_from(2 as u8).unwrap(),
            Self::Volume => u7::try_from(7 as u8).unwrap(),
            Self::Pan => u7::try_from(10 as u8).unwrap(),
        }
    }
}
