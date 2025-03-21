use rinex::prelude::{Constellation, Duration, Observable, TimeScale};
use ublox::{cfg_val::CfgVal, CfgLayer, CfgValSetBuilder};

#[derive(Clone)]
pub struct Settings {
    pub timescale: TimeScale,
    pub sampling_period: Duration,
    pub solutions_ratio: u16,
    pub constellations: Vec<Constellation>,
    pub observables: Vec<Observable>,
    pub sn: Option<String>,
    pub rx_clock: bool,
    pub model: Option<String>,
    pub firmware: Option<String>,
}

impl Settings {
    pub fn to_ram_volatile_cfg(&self, buf: &mut Vec<u8>) {
        let mut cfg_data = Vec::<CfgVal>::new();

        if self.constellations.contains(&Constellation::GPS)
            || self.constellations.contains(&Constellation::QZSS)
        {
            cfg_data.push(CfgVal::SignalGpsEna(true));
            cfg_data.push(CfgVal::SignalGpsL1caEna(true));
            cfg_data.push(CfgVal::SignalGpsL2cEna(true));

            cfg_data.push(CfgVal::SignalQzssEna(true));
        } else {
            cfg_data.push(CfgVal::SignalGpsEna(false));
            cfg_data.push(CfgVal::SignalGpsL1caEna(false));
            cfg_data.push(CfgVal::SignalGpsL2cEna(false));

            cfg_data.push(CfgVal::SignalQzssEna(false));
        }

        if self.constellations.contains(&Constellation::Galileo) {
            cfg_data.push(CfgVal::SignalGalEna(true));
            cfg_data.push(CfgVal::SignalGalE1Ena(true));
            cfg_data.push(CfgVal::SignalGalE5bEna(true));
        } else {
            cfg_data.push(CfgVal::SignalGalEna(false));
            cfg_data.push(CfgVal::SignalGalE1Ena(false));
            cfg_data.push(CfgVal::SignalGalE5bEna(false));
        }

        if self.constellations.contains(&Constellation::QZSS) {
            cfg_data.push(CfgVal::SignalQzssL1caEna(true));
            cfg_data.push(CfgVal::SignalQzssL2cEna(true));
        } else {
            cfg_data.push(CfgVal::SignalQzssL1caEna(false));
            cfg_data.push(CfgVal::SignalQzssL2cEna(false));
        }

        if self.constellations.contains(&Constellation::Glonass) {
            cfg_data.push(CfgVal::SignalGloEna(true));
            cfg_data.push(CfgVal::SignalGloL1Ena(true));
            //cfg_data.push(CfgVal::SignalGloL2Ena(true));
        } else {
            cfg_data.push(CfgVal::SignalGloEna(false));
            cfg_data.push(CfgVal::SignalGloL1Ena(false));
            //cfg_data.push(CfgVal::SignalGloL2Ena(false));
        }

        if self.constellations.contains(&Constellation::BeiDou) {
            cfg_data.push(CfgVal::SignalBdsEna(true));
            cfg_data.push(CfgVal::SignalBdsB1Ena(true));
            cfg_data.push(CfgVal::SignalBdsB2Ena(true));
        } else {
            cfg_data.push(CfgVal::SignalBdsEna(false));
            cfg_data.push(CfgVal::SignalBdsB1Ena(false));
            cfg_data.push(CfgVal::SignalBdsB2Ena(false));
        }

        CfgValSetBuilder {
            version: 0,
            layers: CfgLayer::RAM,
            reserved1: 0,
            cfg_data: &cfg_data,
        }
        .extend_to(buf);
    }
}
