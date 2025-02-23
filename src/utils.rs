use ublox::MonGnssConstellMask;

use rinex::prelude::{Constellation, Observable};

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

    if constellations.contains((&Constellation::BeiDou)) {
        mask |= MonGnssConstellMask::BDC;
    }

    if constellations.contains(&Constellation::Glonass) {
        mask |= MonGnssConstellMask::GLO;
    }

    mask
}

pub fn gnss_id_to_constellation(id: u8) -> Option<Constellation> {
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

// pub fn freq_id_to_observable(constellation: Constellation, freq_id: u8) -> Obseravble {

// }
