use ublox::{AlignmentToReferenceTime, MonGnssConstellMask};

use rinex::prelude::{Carrier, Constellation, TimeScale};

pub fn to_timescale(t_ref: AlignmentToReferenceTime) -> TimeScale {
    match t_ref {
        AlignmentToReferenceTime::Bds => TimeScale::BDT,
        AlignmentToReferenceTime::Gal => TimeScale::GST,
        AlignmentToReferenceTime::Gps => TimeScale::GPST,
        AlignmentToReferenceTime::Utc => TimeScale::UTC,
        AlignmentToReferenceTime::Glo => panic!("GlonassT is not supported yet!"),
    }
}

pub fn from_timescale(ts: TimeScale) -> AlignmentToReferenceTime {
    match ts {
        TimeScale::GPST => AlignmentToReferenceTime::Gps,
        TimeScale::GST => AlignmentToReferenceTime::Gal,
        TimeScale::BDT => AlignmentToReferenceTime::Bds,
        TimeScale::UTC => AlignmentToReferenceTime::Utc,
        ts => panic!("{} timescale is not supported", ts),
    }
}

pub fn constell_mask_to_string(mask: MonGnssConstellMask) -> String {
    let mut string = String::with_capacity(16);
    if mask.intersects(MonGnssConstellMask::GPS) {
        string.push_str("GPS, ");
    }
    if mask.intersects(MonGnssConstellMask::GAL) {
        string.push_str("Galileo, ");
    }
    if mask.intersects(MonGnssConstellMask::BDC) {
        string.push_str("BeiDou, ");
    }
    if mask.intersects(MonGnssConstellMask::GLO) {
        string.push_str("Glonass, ");
    }
    string
}

pub fn constellations_to_mask(constellations: &[Constellation]) -> MonGnssConstellMask {
    let mut mask = MonGnssConstellMask::empty();

    if constellations.contains(&Constellation::GPS) {
        mask |= MonGnssConstellMask::GPS;
    }

    if constellations.contains(&Constellation::Galileo) {
        mask |= MonGnssConstellMask::GAL;
    }

    if constellations.contains(&Constellation::BeiDou) {
        mask |= MonGnssConstellMask::BDC;
    }

    if constellations.contains(&Constellation::Glonass) {
        mask |= MonGnssConstellMask::GLO;
    }

    mask
}

pub fn to_constellation(id: u8) -> Option<Constellation> {
    match id {
        0 => Some(Constellation::GPS),
        1 => Some(Constellation::SBAS),
        2 => Some(Constellation::Galileo),
        3 => Some(Constellation::BeiDou),
        5 => Some(Constellation::QZSS),
        6 => Some(Constellation::Glonass),
        _ => None,
    }
}

pub fn freq_id_to_carrier(constellation: Constellation, freq_id: u8) -> Option<Carrier> {
    match constellation {
        Constellation::GPS => match freq_id {
            0 => Some(Carrier::L1),
            _ => None,
        },
        _ => None,
    }
}
