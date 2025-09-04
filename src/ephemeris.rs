use hifitime::prelude::{Duration, Epoch};

use gnss_protos::{
    GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssHow, GpsQzssSubframe,
};

#[derive(Debug, Default, Copy, Clone)]
pub struct GpsQzssEphemeris {
    pub how: GpsQzssHow,
    pub frame1: GpsQzssFrame1,
    pub frame2: GpsQzssFrame2,
    pub frame3: GpsQzssFrame3,
}

#[derive(Debug, Copy, Clone)]
pub enum Ephemeris {
    GpsQzss(GpsQzssEphemeris),
}

#[derive(Debug, Default, Copy, Clone)]
pub struct PendingGpsQzssFrame {
    pub how: GpsQzssHow,
    pub frame1: Option<GpsQzssFrame1>,
    pub frame2: Option<GpsQzssFrame2>,
    pub frame3: Option<GpsQzssFrame3>,
}

#[derive(Debug, Copy, Clone)]
pub enum PendingFrame {
    GpsQzss(PendingGpsQzssFrame),
}

impl PendingFrame {
    pub fn validate(&self) -> Option<Ephemeris> {
        match self {
            Self::GpsQzss(pending) => {
                let validated = pending.validate()?;
                Some(Ephemeris::GpsQzss(validated))
            },
        }
    }
}

impl PendingGpsQzssFrame {
    pub fn new(frame: GpsQzssFrame) -> Self {
        match frame.subframe {
            GpsQzssSubframe::Ephemeris1(eph1) => Self {
                how: frame.how,
                frame2: None,
                frame3: None,
                frame1: Some(eph1),
            },
            GpsQzssSubframe::Ephemeris2(eph2) => Self {
                how: frame.how,
                frame3: None,
                frame1: None,
                frame2: Some(eph2),
            },
            GpsQzssSubframe::Ephemeris3(eph3) => Self {
                how: frame.how,
                frame2: None,
                frame1: None,
                frame3: Some(eph3),
            },
        }
    }

    pub fn update(&mut self, frame: GpsQzssFrame) {
        self.how = frame.how.clone();

        match frame.subframe {
            GpsQzssSubframe::Ephemeris1(eph1) => {
                self.frame1 = Some(eph1);
            },
            GpsQzssSubframe::Ephemeris2(eph2) => {
                self.frame2 = Some(eph2);
            },
            GpsQzssSubframe::Ephemeris3(eph3) => {
                self.frame3 = Some(eph3);
            },
        }
    }

    pub fn validate(&self) -> Option<GpsQzssEphemeris> {
        let frame1 = self.frame1?;
        let frame2 = self.frame2?;
        let frame3 = self.frame3?;

        if frame2.iode == frame3.iode {
            if frame1.iodc as u8 == frame2.iode {
                return Some(GpsQzssEphemeris {
                    how: self.how,
                    frame1,
                    frame2,
                    frame3,
                });
            }
        }

        None
    }
}
