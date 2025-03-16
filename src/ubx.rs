use rinex::prelude::{Constellation, Duration, Observable, TimeScale};
use ublox::{cfg_val::CfgVal, CfgLayer, CfgMsgAllPortsBuilder, CfgMsgSinglePort, CfgValSetBuilder};

#[derive(Clone)]
pub struct Settings {
    timescale: TimeScale,
    sampling_period: Duration,
    solutions_ratio: u16,
    constellations: Vec<Constellation>,
    observables: Vec<Observable>,
    sn: Option<String>,
    rx_clock: bool,
    model: Option<String>,
    firmware: Option<String>,
}

impl Settings {
    pub fn to_ram_volatile_cfg(&self, buf: &mut [u8]) {
        let mut cfg_data = Vec::<CfgVal>::new();

        let builder = CfgValSetBuilder {
            version: 0,
            layers: CfgLayer::RAM,
            reserved1: 0,
            cfg_data: &cfg_data,
        };

        let bytes: [u8; 16] = builder.into();

        cfg_data.into_cfg_kv_bytes();
        builder.extend_to(&mut buf);
    }
}
