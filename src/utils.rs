use std::str::FromStr;

use ublox::{AlignmentToReferenceTime, MonGnssConstellMask};

use rinex::{
    prelude::{Carrier, Constellation, Duration, Epoch, TimeScale},
    production::{FFU, PPU},
};

use hifitime::{efmt::Format, prelude::Formatter};

pub fn v2_filename(crinex: bool, gzip: bool, t0: Epoch, name: &str) -> String {
    let fmt = Format::from_str("%j").unwrap();

    let (y, _, _, _, _, _, _) = t0.to_gregorian_utc();

    let formatter = Formatter::new(t0, fmt);

    let mut formatted = name.to_string();

    formatted.push_str(&formatter.to_string());
    formatted.push('.');

    formatted.push_str(&format!("{:02}", y - 2000));

    if crinex {
        formatted.push('D');
    } else {
        formatted.push('O');
    }

    if gzip {
        formatted.push_str(".gz")
    }

    formatted
}

pub fn v3_filename(
    crinex: bool,
    gzip: bool,
    name: &str,
    country: &str,
    t0: Epoch,
    period: Duration,
    sampling: Duration,
) -> String {
    let ppu: PPU = period.into();
    let ffu: FFU = sampling.into();

    let mut formatted = format!("{}{}_R_", name, country);

    let fmt = Format::from_str("%Y%j").unwrap();
    let formatter = Formatter::new(t0, fmt);

    formatted.push_str(&formatter.to_string());
    formatted.push_str("0000_");

    formatted.push_str(&ppu.to_string());
    formatted.push('_');

    formatted.push_str(&ffu.to_string());
    formatted.push_str("_MO");

    if crinex {
        formatted.push_str(".crx");
    } else {
        formatted.push_str(".rnx");
    }

    if gzip {
        formatted.push_str(".gz");
    }

    formatted
}

pub fn to_timescale(t_ref: AlignmentToReferenceTime) -> TimeScale {
    match t_ref {
        AlignmentToReferenceTime::Bds => TimeScale::BDT,
        AlignmentToReferenceTime::Gal => TimeScale::GST,
        AlignmentToReferenceTime::Gps => TimeScale::GPST,
        AlignmentToReferenceTime::Utc => TimeScale::UTC,
        AlignmentToReferenceTime::Glo => panic!("GlonassT is not supported yet!"),
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

pub fn freq_id_to_carrier(constellation: Constellation, freq_id: u8) -> Option<Carrier> {
    match constellation {
        Constellation::GPS => match freq_id {
            0 => Some(Carrier::L1),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::{v2_filename, v3_filename};
    use hifitime::prelude::{Duration, Epoch};
    use std::str::FromStr;

    #[test]
    fn test_v2_filename() {
        let t0 = Epoch::from_str("2020-01-01T00:00:00 UTC").unwrap();
        assert_eq!(v2_filename(false, false, t0, "UBX"), "UBX001.20O");
        assert_eq!(v2_filename(true, false, t0, "UBX"), "UBX001.20D");
        assert_eq!(v2_filename(true, true, t0, "UBX"), "UBX001.20D.gz");
    }

    #[test]
    fn test_v3_filename() {
        let t0 = Epoch::from_str("2020-01-01T00:00:00 UTC").unwrap();
        let period = Duration::from_str("1 day").unwrap();
        let sampling = Duration::from_str("30 s").unwrap();
        assert_eq!(
            v3_filename(false, false, "UBX", "USA", t0, period, sampling),
            "UBXUSA_R_20200010000_01D_30S_MO.rnx"
        );
        assert_eq!(
            v3_filename(true, false, "UBX", "USA", t0, period, sampling),
            "UBXUSA_R_20200010000_01D_30S_MO.crx"
        );
        assert_eq!(
            v3_filename(true, true, "UBX", "USA", t0, period, sampling),
            "UBXUSA_R_20200010000_01D_30S_MO.crx.gz"
        );
    }
}
