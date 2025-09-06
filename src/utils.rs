use ublox::AlignmentToReferenceTime;

use rinex::prelude::{Constellation, TimeScale};

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum SignalCarrier {
    #[default]
    GPS_L1_CA,
    GPS_L2_CL,
    GPS_L2_CM,
    GPS_L5_I,
    GPS_L5_Q,
    SBAS_L1_CA,
    GAL_E1_C,
    GAL_E1_B,
    GAL_E5A_I,
    GAL_E5A_Q,
    GAL_E5B_I,
    GAL_E5B_Q,
    BDS_B1I_D1,
    BDS_B1I_D2,
    BDS_B2I_D1,
    BDS_B2I_D2,
    BDS_B1C,
    BDS_B2A,
    QZSS_L1_CA,
    QZSS_L1_S,
    QZSS_L2_CM,
    QZSS_L2_CL,
    QZSS_L5_I,
    QZSS_L5_Q,
    GLO_L1_OF,
    GLO_L2_OF,
    NAVIC_L5_A,
}

impl SignalCarrier {
    pub fn from_ubx(constell_id: u8, freq_id: u8) -> SignalCarrier {
        match (constell_id, freq_id) {
            (0, 3) => Self::GPS_L2_CL,
            (0, 4) => Self::GPS_L2_CM,
            (0, 6) => Self::GPS_L5_I,
            (0, 7) => Self::GPS_L5_Q,
            (1, 0) => Self::SBAS_L1_CA,
            (2, 0) => Self::GAL_E1_C,
            (2, 1) => Self::GAL_E1_B,
            (2, 3) => Self::GAL_E5A_I,
            (2, 4) => Self::GAL_E5A_Q,
            (2, 5) => Self::GAL_E5B_I,
            (2, 6) => Self::GAL_E5B_Q,
            (3, 0) => Self::BDS_B1I_D1,
            (3, 1) => Self::BDS_B1I_D2,
            (3, 2) => Self::BDS_B2I_D1,
            (3, 3) => Self::BDS_B2I_D2,
            (3, 5) => Self::BDS_B1C,
            (3, 7) => Self::BDS_B2A,
            (5, 0) => Self::QZSS_L1_CA,
            (5, 1) => Self::QZSS_L1_S,
            (5, 4) => Self::QZSS_L2_CM,
            (5, 5) => Self::QZSS_L2_CL,
            (5, 8) => Self::QZSS_L5_I,
            (5, 9) => Self::QZSS_L5_Q,
            (6, 0) => Self::GLO_L1_OF,
            (6, 2) => Self::GLO_L2_OF,
            (7, 0) => Self::NAVIC_L5_A,
            _ => Self::default(),
        }
    }

    pub fn to_pseudo_range_observable(&self, v2: bool) -> String {
        if v2 {
            match self {
                Self::GPS_L1_CA
                | Self::SBAS_L1_CA
                | Self::GAL_E1_C
                | Self::GAL_E1_B
                | Self::QZSS_L1_CA
                | Self::GLO_L1_OF
                | Self::BDS_B1I_D1
                | Self::BDS_B1I_D2
                | Self::BDS_B2I_D1
                | Self::BDS_B2I_D2 => "C1".to_string(),
                Self::GPS_L2_CL
                | Self::GPS_L2_CM
                | Self::QZSS_L2_CM
                | Self::QZSS_L2_CL
                | Self::GLO_L2_OF => "C2".to_string(),
                Self::GPS_L5_I
                | Self::GPS_L5_Q
                | Self::GAL_E5A_I
                | Self::GAL_E5A_Q
                | Self::QZSS_L5_I
                | Self::QZSS_L5_Q
                | Self::NAVIC_L5_A => "C5".to_string(),
                Self::GAL_E5B_I | Self::GAL_E5B_Q => "C7".to_string(),
                _ => "C1".to_string(),
            }
        } else {
            match self {
                Self::GPS_L1_CA => "C1C".to_string(),
                Self::GPS_L2_CL => "C2L".to_string(),
                Self::GPS_L2_CM => "C2S".to_string(),
                Self::GPS_L5_I => "C5I".to_string(),
                Self::GPS_L5_Q => "C5Q".to_string(),
                Self::SBAS_L1_CA => "C1C".to_string(),
                Self::GAL_E1_C => "C1C".to_string(),
                Self::GAL_E1_B => "C1B".to_string(),
                Self::GAL_E5A_I => "C5I".to_string(),
                Self::GAL_E5A_Q => "C5Q".to_string(),
                Self::GAL_E5B_I => "C7I".to_string(),
                Self::GAL_E5B_Q => "C7Q".to_string(),
                Self::BDS_B1I_D1 => "C2I".to_string(),
                Self::BDS_B1I_D2 => "C1D".to_string(),
                Self::BDS_B2I_D1 => "C5D".to_string(),
                Self::BDS_B2I_D2 => "C7I".to_string(),
                Self::BDS_B1C => "C5D".to_string(),
                Self::BDS_B2A => "C5D".to_string(),
                Self::QZSS_L1_CA => "C1C".to_string(),
                Self::QZSS_L1_S => "C1Z".to_string(),
                Self::QZSS_L2_CM => "C2S".to_string(),
                Self::QZSS_L2_CL => "C2L".to_string(),
                Self::QZSS_L5_I => "C5I".to_string(),
                Self::QZSS_L5_Q => "C5Q".to_string(),
                Self::GLO_L1_OF => "C1C".to_string(),
                Self::GLO_L2_OF => "C2C".to_string(),
                Self::NAVIC_L5_A => "C1C".to_string(),
            }
        }
    }

    pub fn to_phase_range_observable(&self, v2: bool) -> String {
        if v2 {
            match self {
                Self::GPS_L1_CA
                | Self::SBAS_L1_CA
                | Self::GAL_E1_C
                | Self::GAL_E1_B
                | Self::QZSS_L1_CA
                | Self::GLO_L1_OF
                | Self::BDS_B1I_D1
                | Self::BDS_B1I_D2
                | Self::BDS_B2I_D1
                | Self::BDS_B2I_D2 => "C1".to_string(),
                Self::GPS_L2_CL
                | Self::GPS_L2_CM
                | Self::QZSS_L2_CM
                | Self::QZSS_L2_CL
                | Self::GLO_L2_OF => "C2".to_string(),
                Self::GPS_L5_I
                | Self::GPS_L5_Q
                | Self::GAL_E5A_I
                | Self::GAL_E5A_Q
                | Self::QZSS_L5_I
                | Self::QZSS_L5_Q
                | Self::NAVIC_L5_A => "C5".to_string(),
                Self::GAL_E5B_I | Self::GAL_E5B_Q => "C7".to_string(),
                _ => "C1".to_string(),
            }
        } else {
            match self {
                Self::GPS_L1_CA => "L1C".to_string(),
                Self::GPS_L2_CL => "L2L".to_string(),
                Self::GPS_L2_CM => "L2S".to_string(),
                Self::GPS_L5_I => "L5I".to_string(),
                Self::GPS_L5_Q => "L5Q".to_string(),
                Self::SBAS_L1_CA => "L1C".to_string(),
                Self::GAL_E1_C => "L1C".to_string(),
                Self::GAL_E1_B => "L1B".to_string(),
                Self::GAL_E5A_I => "L5I".to_string(),
                Self::GAL_E5A_Q => "L5Q".to_string(),
                Self::GAL_E5B_I => "L7I".to_string(),
                Self::GAL_E5B_Q => "L7Q".to_string(),
                Self::BDS_B1I_D1 => "L1I".to_string(),
                Self::BDS_B1I_D2 => "L1D".to_string(),
                Self::BDS_B2I_D1 => "L1C".to_string(),
                Self::BDS_B2I_D2 => "L1C".to_string(),
                Self::BDS_B1C => "L1C".to_string(),
                Self::BDS_B2A => "L5D".to_string(),
                Self::QZSS_L1_CA => "L1C".to_string(),
                Self::QZSS_L1_S => "L1Z".to_string(),
                Self::QZSS_L2_CM => "L2S".to_string(),
                Self::QZSS_L2_CL => "L2L".to_string(),
                Self::QZSS_L5_I => "L5I".to_string(),
                Self::QZSS_L5_Q => "L5Q".to_string(),
                Self::GLO_L1_OF => "L1C".to_string(),
                Self::GLO_L2_OF => "L2C".to_string(),
                Self::NAVIC_L5_A => "L1C".to_string(),
            }
        }
    }

    pub fn to_doppler_observable(&self, v2: bool) -> String {
        if v2 {
            match self {
                Self::GPS_L1_CA
                | Self::SBAS_L1_CA
                | Self::GAL_E1_C
                | Self::GAL_E1_B
                | Self::QZSS_L1_CA
                | Self::GLO_L1_OF
                | Self::BDS_B1I_D1
                | Self::BDS_B1I_D2
                | Self::BDS_B2I_D1
                | Self::BDS_B2I_D2 => "D1".to_string(),
                Self::GPS_L2_CL
                | Self::GPS_L2_CM
                | Self::QZSS_L2_CM
                | Self::QZSS_L2_CL
                | Self::GLO_L2_OF => "D2".to_string(),
                Self::GPS_L5_I
                | Self::GPS_L5_Q
                | Self::GAL_E5A_I
                | Self::GAL_E5A_Q
                | Self::QZSS_L5_I
                | Self::QZSS_L5_Q
                | Self::NAVIC_L5_A => "D5".to_string(),
                Self::GAL_E5B_I | Self::GAL_E5B_Q => "D7".to_string(),
                _ => "D1".to_string(),
            }
        } else {
            match self {
                Self::GPS_L1_CA => "D1C".to_string(),
                Self::GPS_L2_CL => "D2L".to_string(),
                Self::GPS_L2_CM => "D2S".to_string(),
                Self::GPS_L5_I => "D5I".to_string(),
                Self::GPS_L5_Q => "D5Q".to_string(),
                Self::SBAS_L1_CA => "D1C".to_string(),
                Self::GAL_E1_C => "D1C".to_string(),
                Self::GAL_E1_B => "D1B".to_string(),
                Self::GAL_E5A_I => "D5I".to_string(),
                Self::GAL_E5A_Q => "D5Q".to_string(),
                Self::GAL_E5B_I => "D7I".to_string(),
                Self::GAL_E5B_Q => "D7Q".to_string(),
                Self::BDS_B1I_D1 => "D1I".to_string(),
                Self::BDS_B1I_D2 => "D1D".to_string(),
                Self::BDS_B2I_D1 => "D1C".to_string(),
                Self::BDS_B2I_D2 => "D1C".to_string(),
                Self::BDS_B1C => "D1C".to_string(),
                Self::BDS_B2A => "D5D".to_string(),
                Self::QZSS_L1_CA => "D1C".to_string(),
                Self::QZSS_L1_S => "D1Z".to_string(),
                Self::QZSS_L2_CM => "D2S".to_string(),
                Self::QZSS_L2_CL => "D2L".to_string(),
                Self::QZSS_L5_I => "D5I".to_string(),
                Self::QZSS_L5_Q => "D5Q".to_string(),
                Self::GLO_L1_OF => "D1C".to_string(),
                Self::GLO_L2_OF => "D2C".to_string(),
                Self::NAVIC_L5_A => "D1C".to_string(),
            }
        }
    }

    pub fn to_ssi_observable(&self, v2: bool) -> String {
        if v2 {
            match self {
                Self::GPS_L1_CA
                | Self::SBAS_L1_CA
                | Self::GAL_E1_C
                | Self::GAL_E1_B
                | Self::QZSS_L1_CA
                | Self::GLO_L1_OF
                | Self::BDS_B1I_D1
                | Self::BDS_B1I_D2
                | Self::BDS_B2I_D1
                | Self::BDS_B2I_D2 => "C1".to_string(),
                Self::GPS_L2_CL
                | Self::GPS_L2_CM
                | Self::QZSS_L2_CM
                | Self::QZSS_L2_CL
                | Self::GLO_L2_OF => "C2".to_string(),
                Self::GPS_L5_I
                | Self::GPS_L5_Q
                | Self::GAL_E5A_I
                | Self::GAL_E5A_Q
                | Self::QZSS_L5_I
                | Self::QZSS_L5_Q
                | Self::NAVIC_L5_A => "C5".to_string(),
                Self::GAL_E5B_I | Self::GAL_E5B_Q => "C7".to_string(),
                _ => "C1".to_string(),
            }
        } else {
            match self {
                Self::GPS_L1_CA => "S1C".to_string(),
                Self::GPS_L2_CL => "S2L".to_string(),
                Self::GPS_L2_CM => "S2S".to_string(),
                Self::GPS_L5_I => "S5I".to_string(),
                Self::GPS_L5_Q => "S5Q".to_string(),
                Self::SBAS_L1_CA => "S1C".to_string(),
                Self::GAL_E1_C => "S1C".to_string(),
                Self::GAL_E1_B => "S1B".to_string(),
                Self::GAL_E5A_I => "S5I".to_string(),
                Self::GAL_E5A_Q => "S5Q".to_string(),
                Self::GAL_E5B_I => "S7I".to_string(),
                Self::GAL_E5B_Q => "S7Q".to_string(),
                Self::BDS_B1I_D1 => "S1I".to_string(),
                Self::BDS_B1I_D2 => "S1D".to_string(),
                Self::BDS_B2I_D1 => "S5D".to_string(),
                Self::BDS_B2I_D2 => "S7I".to_string(),
                Self::BDS_B1C => "S1C".to_string(),
                Self::BDS_B2A => "S5D".to_string(),
                Self::QZSS_L1_CA => "S1C".to_string(),
                Self::QZSS_L1_S => "S1Z".to_string(),
                Self::QZSS_L2_CM => "S2S".to_string(),
                Self::QZSS_L2_CL => "S2L".to_string(),
                Self::QZSS_L5_I => "S5I".to_string(),
                Self::QZSS_L5_Q => "S5Q".to_string(),
                Self::GLO_L1_OF => "S1C".to_string(),
                Self::GLO_L2_OF => "S2C".to_string(),
                Self::NAVIC_L5_A => "S1C".to_string(),
            }
        }
    }
}

//
// pub fn to_timescale(t_ref: AlignmentToReferenceTime) -> TimeScale {
//     match t_ref {
//         AlignmentToReferenceTime::Bds => TimeScale::BDT,
//         AlignmentToReferenceTime::Gal => TimeScale::GST,
//         AlignmentToReferenceTime::Gps => TimeScale::GPST,
//         AlignmentToReferenceTime::Utc => TimeScale::UTC,
//         AlignmentToReferenceTime::Glo => panic!("GlonassT is not supported yet!"),
//     }
// }

pub fn from_timescale(ts: TimeScale) -> AlignmentToReferenceTime {
    match ts {
        TimeScale::GPST => AlignmentToReferenceTime::Gps,
        TimeScale::GST => AlignmentToReferenceTime::Gal,
        TimeScale::BDT => AlignmentToReferenceTime::Bds,
        TimeScale::UTC => AlignmentToReferenceTime::Utc,
        ts => panic!("{} timescale is not supported", ts),
    }
}

// pub fn constell_mask_to_string(mask: MonGnssConstellMask) -> String {
//     let mut string = String::with_capacity(16);
//     if mask.intersects(MonGnssConstellMask::GPS) {
//         string.push_str("GPS, ");
//     }
//     if mask.intersects(MonGnssConstellMask::GAL) {
//         string.push_str("Galileo, ");
//     }
//     if mask.intersects(MonGnssConstellMask::BDC) {
//         string.push_str("BeiDou, ");
//     }
//     if mask.intersects(MonGnssConstellMask::GLO) {
//         string.push_str("Glonass, ");
//     }
//     string
// }

// pub fn constellations_to_mask(constellations: &[Constellation]) -> MonGnssConstellMask {
//     let mut mask = MonGnssConstellMask::empty();

//     if constellations.contains(&Constellation::GPS) {
//         mask |= MonGnssConstellMask::GPS;
//     }

//     if constellations.contains(&Constellation::Galileo) {
//         mask |= MonGnssConstellMask::GAL;
//     }

//     if constellations.contains(&Constellation::BeiDou) {
//         mask |= MonGnssConstellMask::BDC;
//     }

//     if constellations.contains(&Constellation::Glonass) {
//         mask |= MonGnssConstellMask::GLO;
//     }

//     mask
// }

pub fn to_constellation(id: u8) -> Option<Constellation> {
    match id {
        0 => Some(Constellation::GPS),
        1 => Some(Constellation::SBAS),
        2 => Some(Constellation::Galileo),
        3 => Some(Constellation::BeiDou),
        5 => Some(Constellation::QZSS),
        6 => Some(Constellation::Glonass),
        7 => Some(Constellation::IRNSS),
        _ => None,
    }
}

pub fn from_constellation(constellation: &Constellation) -> u8 {
    match constellation {
        Constellation::SBAS => 1,
        Constellation::Galileo => 2,
        Constellation::BeiDou => 3,
        Constellation::QZSS => 5,
        Constellation::Glonass => 6,
        Constellation::IRNSS => 7,
        _ => 0,
    }
}

// pub fn freq_id_to_carrier(constellation: Constellation, freq_id: u8) -> Option<Carrier> {
//     match constellation {
//         Constellation::GPS => match freq_id {
//             0 => Some(Carrier::L1),
//             _ => None,
//         },
//         _ => None,
//     }
// }
