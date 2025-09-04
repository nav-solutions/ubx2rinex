use hifitime::prelude::{Epoch, TimeScale};

use gnss_protos::{
    GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssHow, GpsQzssSubframe,
};

use ublox::RxmSfrbxInterpreted;

use rinex::navigation::{Ephemeris as RINEX, OrbitItem};

use std::collections::HashMap;

// use crate::runtime::Runtime;

#[derive(Debug, Default, Copy, Clone)]
pub struct GpsQzssEphemeris {
    pub how: GpsQzssHow,
    pub frame1: GpsQzssFrame1,
    pub frame2: GpsQzssFrame2,
    pub frame3: GpsQzssFrame3,
}

impl GpsQzssEphemeris {
    /// Converts [Ephemeris] to (Epoch=ToC, [RINEX])
    pub fn to_rinex(&self) -> (Epoch, RINEX) {
        let toc = Epoch::from_time_of_week(
            self.frame1.week as u32,
            self.frame1.toc as u64,
            TimeScale::GPST,
        );

        (
            toc,
            RINEX {
                clock_bias: self.frame1.af0,
                clock_drift: self.frame1.af1,
                clock_drift_rate: self.frame1.af2,
                orbits: HashMap::from_iter(
                    [
                        ("week".to_string(), OrbitItem::F64(0.0)),
                        ("tgd".to_string(), OrbitItem::F64(self.frame1.tgd)),
                        ("iodc".to_string(), OrbitItem::F64(self.frame1.iodc as f64)),
                        ("toe".to_string(), OrbitItem::F64(self.frame2.toe as f64)),
                        ("m0".to_string(), OrbitItem::F64(self.frame2.m0)),
                        ("deltaN".to_string(), OrbitItem::F64(self.frame2.dn)),
                        ("cuc".to_string(), OrbitItem::F64(self.frame2.cuc)),
                        ("cus".to_string(), OrbitItem::F64(self.frame2.cus)),
                        ("crs".to_string(), OrbitItem::F64(self.frame2.crs)),
                        ("e".to_string(), OrbitItem::F64(self.frame2.e)),
                        ("sqrta".to_string(), OrbitItem::F64(self.frame2.sqrt_a)),
                        ("cic".to_string(), OrbitItem::F64(self.frame3.cic)),
                        ("cis".to_string(), OrbitItem::F64(self.frame3.cis)),
                        ("crc".to_string(), OrbitItem::F64(self.frame3.crc)),
                        ("i0".to_string(), OrbitItem::F64(self.frame3.i0)),
                        ("iode".to_string(), OrbitItem::F64(self.frame3.iode as f64)),
                        ("idot".to_string(), OrbitItem::F64(self.frame3.idot)),
                        ("omega0".to_string(), OrbitItem::F64(self.frame3.omega0)),
                        ("omega".to_string(), OrbitItem::F64(self.frame3.omega)),
                        (
                            "omegaDot".to_string(),
                            OrbitItem::F64(self.frame3.omega_dot),
                        ),
                        //("t_tm".to_string(), OrbitItem::F64(self.frame2.fit_int_flag)),
                        //("fitInt".to_string(), OrbitItem::F64(self.frame2.fit_int_flag)),
                        //("aodo".to_string(), OrbitItem::F64(self.frame2.aodo)),
                        //("ura".to_string(), OrbitItem::F64(self.frame1.ura))
                        //("health".to_string(), OrbitItem::HealthFlag(self.frame1.health))
                        //("l2Codes".to_string(), OrbitItem::F64(self.frame1.l2_p_data_flag))
                        //("reserved4".to_string(), OrbitItem::F64(self.frame1.reserved_word4))
                        //("reserved5".to_string(), OrbitItem::F64(self.frame1.reserved_word5))
                        //("reserved6".to_string(), OrbitItem::F64(self.frame1.reserved_word6))
                        //("reserved7".to_string(), OrbitItem::F64(self.frame1.reserved_word7))
                    ]
                    .into_iter(),
                ),
            },
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Ephemeris {
    GpsQzss(GpsQzssEphemeris),
}

impl Ephemeris {
    /// Converts [Ephemeris] to (Epoch=ToC, [RINEX])
    pub fn to_rinex(&self) -> (Epoch, RINEX) {
        match self {
            Self::GpsQzss(ephemeris) => ephemeris.to_rinex(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct PendingGpsQzssFrame {
    pub how: GpsQzssHow,
    pub frame1: Option<GpsQzssFrame1>,
    pub frame2: Option<GpsQzssFrame2>,
    pub frame3: Option<GpsQzssFrame3>,
}

impl PendingGpsQzssFrame {
    pub fn update(&mut self, frame: GpsQzssFrame) {
        self.how = frame.how;
        match frame.subframe {
            GpsQzssSubframe::Ephemeris1(subframe) => {
                self.frame1 = Some(subframe);
            },
            GpsQzssSubframe::Ephemeris2(subframe) => {
                self.frame2 = Some(subframe);
            },
            GpsQzssSubframe::Ephemeris3(subframe) => {
                self.frame3 = Some(subframe);
            },
        }
    }

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

    pub fn update(&mut self, interpretation: RxmSfrbxInterpreted) {
        match (self, interpretation) {
            (Self::GpsQzss(pending), RxmSfrbxInterpreted::GpsQzss(frame)) => pending.update(frame),
            _ => {}, // either unhandled or invalid combination
        }
    }
}
