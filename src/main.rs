#![doc(
    html_logo_url = "https://raw.githubusercontent.com/nav-solutions/.github/master/logos/logo2.jpg"
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::type_complexity)]

/*
 * UBX2RINEX is part of the nav-solutions framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al,
 * (cf. https://github.com/nav-solutions/rinex/graphs/contributors)
 * (cf. https://github.com/nav-solutions/ubx2rinex/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation: https://github.com/nav-solutions/ubx2rinex
 */

extern crate gnss_rs as gnss;
extern crate ublox;

#[cfg(feature = "proto23")]
pub(crate) type Proto = ublox::proto23::Proto23;

#[cfg(all(feature = "proto27", not(feature = "proto23")))]
pub(crate) type Proto = ublox::proto27::Proto27;

#[cfg(all(
    feature = "proto31",
    not(any(feature = "proto23", feature = "proto27"))
))]
pub(crate) type Proto = ublox::proto31::Proto31;

use itertools::Itertools;

use env_logger::{Builder, Target};

use log::{debug, error, info, trace, warn};

use tokio::{
    signal,
    sync::{mpsc, watch},
};

use std::fs::File;

use rinex::prelude::{Constellation, Duration, Epoch, SV, TimeScale};

use ublox::{
    UbxPacket, nav_pvt::common::NavPvtValidFlags, nav_time_utc::NavTimeUtcFlags,
    rxm_rawx::RecStatFlags,
};

#[cfg(feature = "proto23")]
use ublox::packetref_proto23::PacketRef;

#[cfg(all(feature = "proto27", not(feature = "proto23")))]
use ublox::packetref_proto27::PacketRef;

#[cfg(all(
    feature = "proto31",
    not(any(feature = "proto23", feature = "proto27"))
))]
use ublox::packetref_proto31::PacketRef;

mod cli;
mod collecter;
mod device;
mod runtime;
mod ubx;
mod utils;

use crate::{
    cli::Cli,
    collecter::{
        Message, navigation::Collecter as NavCollecter, observation::Collecter as ObsCollecter,
        rawxm::Rawxm,
    },
    device::Device,
    runtime::Runtime,
    ubx::Settings as UbloxSettings,
    utils::to_constellation,
};

const SBAS_PRN_OFFSET: u8 = 100;

fn consume_device(
    runtime: &mut Runtime,
    obs_tx: &mut mpsc::Sender<Message>,
    nav_tx: &mut mpsc::Sender<Message>,
    device: &mut Device<Proto>,
    buffer: &mut [u8],
    cfg_precision: Duration,
    ubx_settings: &UbloxSettings,
) -> std::io::Result<usize> {
    let mut end_of_nav_epoch = false;

    device.consume_all_cb(buffer, |packet| {
        match packet {
            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::CfgNav5(_)) => {
                // TODO: Dynamic model ?
                // let _dyn_model = pkt.dyn_model();
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::CfgNav5(_)) => {
                // TODO: Dynamic model ?
                // let _dyn_model = pkt.dyn_model();
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::CfgNav5(_)) => {
                // TODO: Dynamic model ?
                // let _dyn_model = pkt.dyn_model();
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::RxmSfrbx(sfrbx)) => {
                // Do not process if user is not interested in this channel.
                // When attached to hardware this naturally never happens.
                // But this may arise in passive mode.
                if ubx_settings.ephemeris {
                    let gnss_id = sfrbx.gnss_id();

                    match to_constellation(gnss_id) {
                        Some(constellation) => {
                            // does not proceeed if we're not interested by this system
                            if ubx_settings.constellations.contains(&constellation) {
                                let mut prn = sfrbx.sv_id();

                                if constellation.is_sbas() && prn >= SBAS_PRN_OFFSET {
                                    prn -= SBAS_PRN_OFFSET;
                                }

                                let sv = SV::new(constellation, prn);

                                match constellation {
                                    Constellation::GPS | Constellation::QZSS => {
                                        // decode
                                        if let Some(interpretation) = sfrbx.interpret() {
                                            debug!(
                                                "{} - decoded {:?}",
                                                runtime.utc_time().round(cfg_precision),
                                                interpretation
                                            );

                                            runtime.latch_sfrbx(sv, interpretation, cfg_precision);
                                        } else {
                                            error!(
                                                "{} - SFRBX interpretation issue",
                                                runtime.utc_time().round(cfg_precision)
                                            );
                                        }
                                    },
                                    c => {
                                        error!(
                                            "{} - {} constellation not handled yet",
                                            runtime.utc_time().round(cfg_precision),
                                            c
                                        );
                                    },
                                }
                            }
                        },
                        None => {
                            error!(
                                "{} - constellation id error #{}",
                                runtime.utc_time().round(cfg_precision),
                                gnss_id
                            );
                        },
                    }
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::RxmSfrbx(sfrbx)) => {
                // Do not process if user is not interested in this channel.
                // When attached to hardware this naturally never happens.
                // But this may arise in passive mode.
                if ubx_settings.ephemeris {
                    let gnss_id = sfrbx.gnss_id();

                    match to_constellation(gnss_id) {
                        Some(constellation) => {
                            // does not proceeed if we're not interested by this system
                            if ubx_settings.constellations.contains(&constellation) {
                                let mut prn = sfrbx.sv_id();

                                if constellation.is_sbas() && prn >= SBAS_PRN_OFFSET {
                                    prn -= SBAS_PRN_OFFSET;
                                }

                                let sv = SV::new(constellation, prn);

                                match constellation {
                                    Constellation::GPS | Constellation::QZSS => {
                                        // decode
                                        if let Some(interpretation) = sfrbx.interpret() {
                                            debug!(
                                                "{} - decoded {:?}",
                                                runtime.utc_time().round(cfg_precision),
                                                interpretation
                                            );

                                            runtime.latch_sfrbx(sv, interpretation, cfg_precision);
                                        } else {
                                            error!(
                                                "{} - SFRBX interpretation issue",
                                                runtime.utc_time().round(cfg_precision)
                                            );
                                        }
                                    },
                                    c => {
                                        error!(
                                            "{} - {} constellation not handled yet",
                                            runtime.utc_time().round(cfg_precision),
                                            c
                                        );
                                    },
                                }
                            }
                        },
                        None => {
                            error!(
                                "{} - constellation id error #{}",
                                runtime.utc_time().round(cfg_precision),
                                gnss_id
                            );
                        },
                    }
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::RxmSfrbx(sfrbx)) => {
                // Do not process if user is not interested in this channel.
                // When attached to hardware this naturally never happens.
                // But this may arise in passive mode.
                if ubx_settings.ephemeris {
                    let gnss_id = sfrbx.gnss_id();

                    match to_constellation(gnss_id) {
                        Some(constellation) => {
                            // does not proceeed if we're not interested by this system
                            if ubx_settings.constellations.contains(&constellation) {
                                let mut prn = sfrbx.sv_id();

                                if constellation.is_sbas() && prn >= SBAS_PRN_OFFSET {
                                    prn -= SBAS_PRN_OFFSET;
                                }

                                let sv = SV::new(constellation, prn);

                                match constellation {
                                    Constellation::GPS | Constellation::QZSS => {
                                        // decode
                                        if let Some(interpretation) = sfrbx.interpret() {
                                            debug!(
                                                "{} - decoded {:?}",
                                                runtime.utc_time().round(cfg_precision),
                                                interpretation
                                            );

                                            runtime.latch_sfrbx(sv, interpretation, cfg_precision);
                                        } else {
                                            error!(
                                                "{} - SFRBX interpretation issue",
                                                runtime.utc_time().round(cfg_precision)
                                            );
                                        }
                                    },
                                    c => {
                                        error!(
                                            "{} - {} constellation not handled yet",
                                            runtime.utc_time().round(cfg_precision),
                                            c
                                        );
                                    },
                                }
                            }
                        },
                        None => {
                            error!(
                                "{} - constellation id error #{}",
                                runtime.utc_time().round(cfg_precision),
                                gnss_id
                            );
                        },
                    }
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::RxmRawx(pkt)) => {
                // Do not process if user is not interested in this channel.
                // When attached to hardware this naturally never happens.
                // But this may arise in passive mode.
                if ubx_settings.rawxm {
                    let gpst_tow_nanos = (pkt.rcv_tow() * 1.0E9).round() as u64;

                    let t_gpst = Epoch::from_time_of_week(
                        pkt.week() as u32,
                        gpst_tow_nanos,
                        TimeScale::GPST,
                    );

                    runtime.new_epoch(t_gpst, ubx_settings.timescale);

                    let stat = pkt.rec_stat();

                    if stat.intersects(RecStatFlags::CLK_RESET) {
                        error!("{} - clock reset!", t_gpst.round(cfg_precision));

                        warn!(
                            "{} - declaring phase cycle slip! - !!case is not handled!!",
                            t_gpst.round(cfg_precision)
                        );

                        error!(
                            "{} - phase cycle slip not correctly managed in current version",
                            t_gpst.round(cfg_precision)
                        );
                    }

                    for meas in pkt.measurements() {
                        let pr = meas.pr_mes();
                        let cp = meas.cp_mes();
                        let dop = meas.do_mes();

                        let _ = meas.pr_stdev(); // CXX deviation
                        let _ = meas.cp_stdev(); // LXX deviation
                        let _ = meas.do_stdev(); // DXX deviation

                        let gnss_id = meas.gnss_id();
                        let cno = meas.cno();

                        let constell = to_constellation(gnss_id);

                        if constell.is_none() {
                            error!(
                                "{} - unknown constellation: #{}",
                                runtime.utc_time().round(cfg_precision),
                                gnss_id
                            );
                            continue;
                        }

                        let constell = constell.unwrap();

                        // does not proceed if we're not interested by this system
                        if ubx_settings.constellations.contains(&constell) {
                            let mut prn = meas.sv_id();

                            if constell.is_sbas() && prn >= SBAS_PRN_OFFSET {
                                prn -= SBAS_PRN_OFFSET;
                            };

                            let sv = SV::new(constell, prn);
                            let t_meas = t_gpst.to_time_scale(ubx_settings.timescale);

                            let rawxm = Rawxm {
                                epoch: t_meas,
                                sv,
                                pr,
                                cp,
                                cno,
                                dop,
                                freq_id: meas.freq_id(),
                            };

                            match obs_tx.try_send(Message::Measurement(rawxm)) {
                                Ok(_) => {},
                                Err(e) => {
                                    error!(
                                        "{}({}) failed to send measurement: {}",
                                        t_meas.round(cfg_precision),
                                        sv,
                                        e
                                    );
                                },
                            }
                        }
                    }
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::RxmRawx(pkt)) => {
                // Do not process if user is not interested in this channel.
                // When attached to hardware this naturally never happens.
                // But this may arise in passive mode.
                if ubx_settings.rawxm {
                    let gpst_tow_nanos = (pkt.rcv_tow() * 1.0E9).round() as u64;

                    let t_gpst = Epoch::from_time_of_week(
                        pkt.week() as u32,
                        gpst_tow_nanos,
                        TimeScale::GPST,
                    );

                    runtime.new_epoch(t_gpst, ubx_settings.timescale);

                    let stat = pkt.rec_stat();

                    if stat.intersects(RecStatFlags::CLK_RESET) {
                        error!("{} - clock reset!", t_gpst.round(cfg_precision));

                        warn!(
                            "{} - declaring phase cycle slip! - !!case is not handled!!",
                            t_gpst.round(cfg_precision)
                        );

                        error!(
                            "{} - phase cycle slip not correctly managed in current version",
                            t_gpst.round(cfg_precision)
                        );
                    }

                    for meas in pkt.measurements() {
                        let pr = meas.pr_mes();
                        let cp = meas.cp_mes();
                        let dop = meas.do_mes();

                        let _ = meas.pr_stdev(); // CXX deviation
                        let _ = meas.cp_stdev(); // LXX deviation
                        let _ = meas.do_stdev(); // DXX deviation

                        let gnss_id = meas.gnss_id();
                        let cno = meas.cno();

                        let constell = to_constellation(gnss_id);

                        if constell.is_none() {
                            error!(
                                "{} - unknown constellation: #{}",
                                runtime.utc_time().round(cfg_precision),
                                gnss_id
                            );
                            continue;
                        }

                        let constell = constell.unwrap();

                        // does not proceed if we're not interested by this system
                        if ubx_settings.constellations.contains(&constell) {
                            let mut prn = meas.sv_id();

                            if constell.is_sbas() && prn >= SBAS_PRN_OFFSET {
                                prn -= SBAS_PRN_OFFSET;
                            };

                            let sv = SV::new(constell, prn);
                            let t_meas = t_gpst.to_time_scale(ubx_settings.timescale);

                            let rawxm = Rawxm {
                                epoch: t_meas,
                                sv,
                                pr,
                                cp,
                                cno,
                                dop,
                                freq_id: meas.freq_id(),
                            };

                            match obs_tx.try_send(Message::Measurement(rawxm)) {
                                Ok(_) => {},
                                Err(e) => {
                                    error!(
                                        "{}({}) failed to send measurement: {}",
                                        t_meas.round(cfg_precision),
                                        sv,
                                        e
                                    );
                                },
                            }
                        }
                    }
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::RxmRawx(pkt)) => {
                // Do not process if user is not interested in this channel.
                // When attached to hardware this naturally never happens.
                // But this may arise in passive mode.
                if ubx_settings.rawxm {
                    let gpst_tow_nanos = (pkt.rcv_tow() * 1.0E9).round() as u64;

                    let t_gpst = Epoch::from_time_of_week(
                        pkt.week() as u32,
                        gpst_tow_nanos,
                        TimeScale::GPST,
                    );

                    runtime.new_epoch(t_gpst, ubx_settings.timescale);

                    let stat = pkt.rec_stat();

                    if stat.intersects(RecStatFlags::CLK_RESET) {
                        error!("{} - clock reset!", t_gpst.round(cfg_precision));

                        warn!(
                            "{} - declaring phase cycle slip! - !!case is not handled!!",
                            t_gpst.round(cfg_precision)
                        );

                        error!(
                            "{} - phase cycle slip not correctly managed in current version",
                            t_gpst.round(cfg_precision)
                        );
                    }

                    for meas in pkt.measurements() {
                        let pr = meas.pr_mes();
                        let cp = meas.cp_mes();
                        let dop = meas.do_mes();

                        let _ = meas.pr_stdev(); // CXX deviation
                        let _ = meas.cp_stdev(); // LXX deviation
                        let _ = meas.do_stdev(); // DXX deviation

                        let gnss_id = meas.gnss_id();
                        let cno = meas.cno();

                        let constell = to_constellation(gnss_id);

                        if constell.is_none() {
                            error!(
                                "{} - unknown constellation: #{}",
                                runtime.utc_time().round(cfg_precision),
                                gnss_id
                            );
                            continue;
                        }

                        let constell = constell.unwrap();

                        // does not proceed if we're not interested by this system
                        if ubx_settings.constellations.contains(&constell) {
                            let mut prn = meas.sv_id();

                            if constell.is_sbas() && prn >= SBAS_PRN_OFFSET {
                                prn -= SBAS_PRN_OFFSET;
                            };

                            let sv = SV::new(constell, prn);
                            let t_meas = t_gpst.to_time_scale(ubx_settings.timescale);

                            let rawxm = Rawxm {
                                epoch: t_meas,
                                sv,
                                pr,
                                cp,
                                cno,
                                dop,
                                freq_id: meas.freq_id(),
                            };

                            match obs_tx.try_send(Message::Measurement(rawxm)) {
                                Ok(_) => {},
                                Err(e) => {
                                    error!(
                                        "{}({}) failed to send measurement: {}",
                                        t_meas.round(cfg_precision),
                                        sv,
                                        e
                                    );
                                },
                            }
                        }
                    }
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::MonVer(mon_version)) => {
                let software_version = mon_version.software_version().to_string();

                match obs_tx.try_send(Message::FirmwareVersion(software_version)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send firmware version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }

                let comment = format!("UBlox hardware version: {}", mon_version.hardware_version());

                match obs_tx.try_send(Message::HeaderComment(comment)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send hardware version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }

                let comment = format!("UBlox protocol: {}", mon_version.extension().join(","));

                match obs_tx.try_send(Message::HeaderComment(comment)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send ublox proto version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::MonVer(mon_version)) => {
                let software_version = mon_version.software_version().to_string();

                match obs_tx.try_send(Message::FirmwareVersion(software_version)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send firmware version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }

                let comment = format!("UBlox hardware version: {}", mon_version.hardware_version());

                match obs_tx.try_send(Message::HeaderComment(comment)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send hardware version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }

                let comment = format!("UBlox protocol: {}", mon_version.extension().join(","));

                match obs_tx.try_send(Message::HeaderComment(comment)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send ublox proto version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::MonVer(pkt)) => {
                let software_version = mon_version.software_version().to_string();

                match obs_tx.try_send(Message::FirmwareVersion(software_version)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send firmware version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }

                let comment = format!("UBlox hardware version: {}", mon_version.hardware_version());

                match obs_tx.try_send(Message::HeaderComment(comment)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send hardware version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }

                let comment = format!("UBlox protocol: {}", mon_version.extension().join(","));

                match obs_tx.try_send(Message::HeaderComment(comment)) {
                    Ok(_) => {},
                    Err(e) => {
                        error!(
                            "{} - failed to send ublox proto version: {}",
                            runtime.utc_time().round(cfg_precision),
                            e
                        );
                    },
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::MonHw(mon_hardware)) => {
                // TODO: should contribute to hardware events
                let _ = mon_hardware.a_status();
                let _ = mon_hardware.a_power();
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::MonHw(mon_hardware)) => {
                // TODO: should contribute to hardware events
                let _ = mon_hardware.a_status();
                let _ = mon_hardware.a_power();
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::MonHw(mon_hardware)) => {
                // TODO: should contribute to hardware events
                let _ = mon_hardware.a_status();
                let _ = mon_hardware.a_power();
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::NavSat(pkt)) => {
                for sv in pkt.svs() {
                    let constellation = to_constellation(sv.gnss_id());

                    if constellation.is_none() {
                        continue;
                    }

                    let constellation = constellation.unwrap();

                    let _elev = sv.elev();
                    let _azim = sv.azim();
                    let _pr_res = sv.pr_res();
                    let _flags = sv.flags();

                    let mut prn = sv.sv_id();

                    if constellation.is_sbas() && prn >= SBAS_PRN_OFFSET {
                        prn -= SBAS_PRN_OFFSET;
                    }

                    // let sv = SV::new(constellation, prn);
                    // flags.sv_used()
                    //flags.health();
                    //flags.quality_ind();
                    //flags.differential_correction_available();
                    //flags.ephemeris_available();
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::NavSat(pkt)) => {
                for sv in pkt.svs() {
                    let constellation = to_constellation(sv.gnss_id());

                    if constellation.is_none() {
                        continue;
                    }

                    let constellation = constellation.unwrap();

                    let _elev = sv.elev();
                    let _azim = sv.azim();
                    let _pr_res = sv.pr_res();
                    let _flags = sv.flags();

                    let mut prn = sv.sv_id();

                    if constellation.is_sbas() && prn >= SBAS_PRN_OFFSET {
                        prn -= SBAS_PRN_OFFSET;
                    }

                    // let sv = SV::new(constellation, prn);
                    // flags.sv_used()
                    //flags.health();
                    //flags.quality_ind();
                    //flags.differential_correction_available();
                    //flags.ephemeris_available();
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::NavSat(pkt)) => {
                for sv in pkt.svs() {
                    let constellation = to_constellation(sv.gnss_id());

                    if constellation.is_none() {
                        continue;
                    }

                    let constellation = constellation.unwrap();

                    let _elev = sv.elev();
                    let _azim = sv.azim();
                    let _pr_res = sv.pr_res();
                    let _flags = sv.flags();

                    let mut prn = sv.sv_id();

                    if constellation.is_sbas() && prn >= SBAS_PRN_OFFSET {
                        prn -= SBAS_PRN_OFFSET;
                    }

                    // let sv = SV::new(constellation, prn);
                    // flags.sv_used()
                    //flags.health();
                    //flags.quality_ind();
                    //flags.differential_correction_available();
                    //flags.ephemeris_available();
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::NavTimeUTC(pkt)) => {
                if pkt.valid().intersects(NavTimeUtcFlags::VALID_UTC) {
                    // leap seconds already known
                    // let e = Epoch::maybe_from_gregorian(
                    //     pkt.year().into(),
                    //     pkt.month(),
                    //     pkt.day(),
                    //     pkt.hour(),
                    //     pkt.min(),
                    //     pkt.sec(),
                    //     pkt.nanos() as u32,
                    //     TimeScale::UTC,
                    // );
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::NavTimeUTC(pkt)) => {
                if pkt.valid().intersects(NavTimeUtcFlags::VALID_UTC) {
                    // leap seconds already known
                    // let e = Epoch::maybe_from_gregorian(
                    //     pkt.year().into(),
                    //     pkt.month(),
                    //     pkt.day(),
                    //     pkt.hour(),
                    //     pkt.min(),
                    //     pkt.sec(),
                    //     pkt.nanos() as u32,
                    //     TimeScale::UTC,
                    // );
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::NavTimeUTC(pkt)) => {
                if pkt.valid().intersects(NavTimeUtcFlags::VALID_UTC) {
                    // leap seconds already known
                    // let e = Epoch::maybe_from_gregorian(
                    //     pkt.year().into(),
                    //     pkt.month(),
                    //     pkt.day(),
                    //     pkt.hour(),
                    //     pkt.min(),
                    //     pkt.sec(),
                    //     pkt.nanos() as u32,
                    //     TimeScale::UTC,
                    // );
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::NavStatus(pkt)) => {
                //itow = pkt.itow();
                runtime.uptime = Duration::from_milliseconds(pkt.uptime_ms() as f64);

                trace!(
                    "Fix status: {:?} | {:?} | {:?}",
                    pkt.fix_stat(),
                    pkt.flags(),
                    pkt.flags2()
                );

                trace!("Uptime: {}", runtime.uptime);
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::NavStatus(pkt)) => {
                //itow = pkt.itow();
                runtime.uptime = Duration::from_milliseconds(pkt.uptime_ms() as f64);

                trace!(
                    "Fix status: {:?} | {:?} | {:?}",
                    pkt.fix_stat(),
                    pkt.flags(),
                    pkt.flags2()
                );

                trace!("Uptime: {}", runtime.uptime);
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::NavStatus(pkt)) => {
                //itow = pkt.itow();
                runtime.uptime = Duration::from_milliseconds(pkt.uptime_ms() as f64);

                trace!(
                    "Fix status: {:?} | {:?} | {:?}",
                    pkt.fix_stat(),
                    pkt.flags(),
                    pkt.flags2()
                );

                trace!("Uptime: {}", runtime.uptime);
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::NavEoe(pkt)) => {
                let gpst_itow_nanos = pkt.itow() as u64 * 1_000_000;
                let t_gpst =
                    Epoch::from_time_of_week(runtime.gpst_week(), gpst_itow_nanos, TimeScale::GPST);
                end_of_nav_epoch = true;
                trace!("{} - End of Epoch", t_gpst.round(cfg_precision));
                let _ = nav_tx.try_send(Message::EndofEpoch());
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::NavEoe(pkt)) => {
                let gpst_itow_nanos = pkt.itow() as u64 * 1_000_000;
                let t_gpst =
                    Epoch::from_time_of_week(runtime.gpst_week(), gpst_itow_nanos, TimeScale::GPST);
                end_of_nav_epoch = true;
                trace!("{} - End of Epoch", t_gpst.round(cfg_precision));
                let _ = nav_tx.try_send(Message::EndofEpoch());
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::NavEoe(pkt)) => {
                let gpst_itow_nanos = pkt.itow() as u64 * 1_000_000;
                let t_gpst =
                    Epoch::from_time_of_week(runtime.gpst_week(), gpst_itow_nanos, TimeScale::GPST);
                end_of_nav_epoch = true;
                trace!("{} - End of Epoch", t_gpst.round(cfg_precision));
                let _ = nav_tx.try_send(Message::EndofEpoch());
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::NavPvt(pkt)) => {
                let (y, m, d) = (pkt.year() as i32, pkt.month(), pkt.day());
                let (hh, mm, ss) = (pkt.hour(), pkt.min(), pkt.sec());

                if pkt.valid().intersects(NavPvtValidFlags::FULLY_RESOLVED) {
                    let t_solution = Epoch::from_gregorian(y, m, d, hh, mm, ss, 0, TimeScale::UTC)
                        .to_time_scale(ubx_settings.timescale);

                    trace!(
                        "{} - PVT SOLUTION: lat={:.5E}° long={:.5E}°",
                        t_solution.round(cfg_precision),
                        pkt.latitude(),
                        pkt.longitude()
                    );
                }
            },
            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::NavPvt(pkt)) => {
                let (y, m, d) = (pkt.year() as i32, pkt.month(), pkt.day());
                let (hh, mm, ss) = (pkt.hour(), pkt.min(), pkt.sec());

                if pkt.valid().intersects(NavPvtValidFlags::FULLY_RESOLVED) {
                    let t_solution = Epoch::from_gregorian(y, m, d, hh, mm, ss, 0, TimeScale::UTC)
                        .to_time_scale(ubx_settings.timescale);

                    trace!(
                        "{} - PVT SOLUTION: lat={:.5E}° long={:.5E}°",
                        t_solution.round(cfg_precision),
                        pkt.latitude(),
                        pkt.longitude()
                    );
                }
            },
            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::NavPvt(pkt)) => {
                let (y, m, d) = (pkt.year() as i32, pkt.month(), pkt.day());
                let (hh, mm, ss) = (pkt.hour(), pkt.min(), pkt.sec());

                if pkt.valid().intersects(NavPvtValidFlags::FULLY_RESOLVED) {
                    let t_solution = Epoch::from_gregorian(y, m, d, hh, mm, ss, 0, TimeScale::UTC)
                        .to_time_scale(ubx_settings.timescale);

                    trace!(
                        "{} - PVT SOLUTION: lat={:.5E}° long={:.5E}°",
                        t_solution.round(cfg_precision),
                        pkt.latitude(),
                        pkt.longitude()
                    );
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::NavClock(pkt)) => {
                // Do not process if user is not interested in this channel.
                if ubx_settings.rawxm && ubx_settings.rx_clock {
                    let clock = pkt.clk_bias();
                    match obs_tx.try_send(Message::Clock(clock)) {
                        Ok(_) => {},
                        Err(e) => {
                            error!(
                                "{} - failed to send clock state: {}",
                                runtime.utc_time().round(cfg_precision),
                                e
                            );
                        },
                    }
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::NavClock(pkt)) => {
                // Do not process if user is not interested in this channel.
                if ubx_settings.rawxm && ubx_settings.rx_clock {
                    let clock = pkt.clk_bias();
                    match obs_tx.try_send(Message::Clock(clock)) {
                        Ok(_) => {},
                        Err(e) => {
                            error!(
                                "{} - failed to send clock state: {}",
                                runtime.utc_time().round(cfg_precision),
                                e
                            );
                        },
                    }
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::NavClock(pkt)) => {
                // Do not process if user is not interested in this channel.
                if ubx_settings.rawxm && ubx_settings.rx_clock {
                    let clock = pkt.clk_bias();
                    match obs_tx.try_send(Message::Clock(clock)) {
                        Ok(_) => {},
                        Err(e) => {
                            error!(
                                "{} - failed to send clock state: {}",
                                runtime.utc_time().round(cfg_precision),
                                e
                            );
                        },
                    }
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::InfTest(pkt)) => {
                if let Some(msg) = pkt.message() {
                    trace!(
                        "{} - received test message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::InfTest(pkt)) => {
                if let Some(msg) = pkt.message() {
                    trace!(
                        "{} - received test message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::InfTest(pkt)) => {
                if let Some(msg) = pkt.message() {
                    trace!(
                        "{} - received test message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::InfDebug(pkt)) => {
                if let Some(msg) = pkt.message() {
                    debug!(
                        "{} - received debug message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::InfDebug(pkt)) => {
                if let Some(msg) = pkt.message() {
                    debug!(
                        "{} - received debug message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::InfDebug(pkt)) => {
                if let Some(msg) = pkt.message() {
                    debug!(
                        "{} - received debug message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::InfNotice(pkt)) => {
                if let Some(msg) = pkt.message() {
                    info!(
                        "{} - received notification {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::InfNotice(pkt)) => {
                if let Some(msg) = pkt.message() {
                    info!(
                        "{} - received notification {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::InfNotice(pkt)) => {
                if let Some(msg) = pkt.message() {
                    info!(
                        "{} - received notification {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::InfError(pkt)) => {
                if let Some(msg) = pkt.message() {
                    error!(
                        "{} - received error notification {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::InfError(pkt)) => {
                if let Some(msg) = pkt.message() {
                    error!(
                        "{} - received error notification {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::InfError(pkt)) => {
                if let Some(msg) = pkt.message() {
                    error!(
                        "{} - received error notification {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto23")]
            ublox::UbxPacket::Proto23(PacketRef::InfWarning(pkt)) => {
                if let Some(msg) = pkt.message() {
                    warn!(
                        "{} - received warning message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto27")]
            ublox::UbxPacket::Proto27(PacketRef::InfWarning(pkt)) => {
                if let Some(msg) = pkt.message() {
                    warn!(
                        "{} - received warning message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },

            #[cfg(feature = "proto31")]
            ublox::UbxPacket::Proto31(PacketRef::InfWarning(pkt)) => {
                if let Some(msg) = pkt.message() {
                    warn!(
                        "{} - received warning message {}",
                        runtime.utc_time().round(cfg_precision),
                        msg
                    );
                }
            },
            _ => {},
        } //packet
    })
}

#[tokio::main]
pub async fn main() {
    // pretty_env_logger::init();
    let mut builder = Builder::from_default_env();

    builder
        .target(Target::Stdout)
        .format_timestamp_secs()
        .format_module_path(false)
        .init();

    // init
    let mut buffer = [0; 8192];

    let cfg_precision = Duration::from_seconds(1.0);

    let t_utc = Epoch::now()
        .unwrap_or_else(|e| panic!("Failed to determine system time: {}", e))
        .to_time_scale(TimeScale::UTC);

    // cli
    let cli = Cli::new();

    // Input interface
    let mut device = if let Some(serial) = cli.serial_port() {
        // active mode (GNSS module)
        let baud_rate = cli.baud_rate().unwrap_or(115_200);
        Device::<Proto>::open_serial_port(serial, baud_rate, &mut buffer)
    } else {
        // passive mode (input files)
        let user_files = cli.filepaths();
        let total = user_files.len();

        assert!(
            total > 0,
            "invalid command line: requires either serial port or at least, one input file"
        );

        let mut device = Device::open_file(user_files[0]);

        for i in 1..total {
            let fd = File::open(user_files[i]).unwrap_or_else(|e| {
                panic!("failed to open {}: {}", user_files[i], e);
            });

            if user_files[i].ends_with(".gz") {
                device.interface.stack_gzip_file_handle(fd);
            } else {
                device.interface.stack_file_handle(fd);
            }
        }

        device
    };

    // RINEX settings
    let settings = cli.rinex_settings();

    // U-Blox settings
    let ubx_settings = cli.ublox_settings();

    // shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(true);

    // Observation RINEX
    let (mut obs_tx, obs_rx) = mpsc::channel(128);

    let mut obs_collecter = ObsCollecter::new(
        settings.clone(),
        ubx_settings.clone(),
        shutdown_rx.clone(),
        obs_rx,
    );

    // Navigation RINEX
    let (mut nav_tx, nav_rx) = mpsc::channel(128);

    let mut nav_collecter = NavCollecter::new(
        settings.clone(),
        ubx_settings.clone(),
        shutdown_rx.clone(),
        nav_rx,
    );

    // Device configuration
    if !device.interface.is_read_only() {
        device.configure(&ubx_settings, &mut buffer, obs_tx.clone());
    }

    // spawns OBS collector
    if ubx_settings.rawxm {
        tokio::spawn(async move {
            info!("{} - Observation mode deployed", t_utc.round(cfg_precision));
            obs_collecter.run().await;
        });
    }

    // spawns NAV collector
    if ubx_settings.ephemeris {
        tokio::spawn(async move {
            info!("{} - Navigation  mode deployed", t_utc.round(cfg_precision));
            nav_collecter.run().await;
        });
    }

    // tokio::spawn(async move {
    //     signal::ctrl_c()
    //         .await
    //         .unwrap_or_else(|e| panic!("Tokio signal handling error: {}", e));

    //     shutdown_tx
    //         .send(true)
    //         .unwrap_or_else(|e| panic!("Tokio: signaling error: {}", e));
    // });

    // main task
    let mut rtm = Runtime::new();
    info!("{} - application deployed", t_utc.round(cfg_precision));

    loop {
        match consume_device(
            &mut rtm,
            &mut obs_tx,
            &mut nav_tx,
            &mut device,
            &mut buffer,
            cfg_precision,
            &ubx_settings,
        ) {
            Ok(0) => {
                // in standard mode, this may happen,
                // in passive mode, we have consumed all content: we should exit.
                if device.interface.is_read_only() {
                    info!(
                        "{} - consumed all content",
                        rtm.utc_time().round(cfg_precision)
                    );

                    break;
                }
            },
            Ok(_) => {}, // nominal
            Err(e) => {
                error!("{} - I/O error: {}", rtm.utc_time().round(cfg_precision), e);
            },
        }

        // handle all pending NAV-EPH messages
        if ubx_settings.ephemeris {
            for (sv, pending) in rtm.pending_frames.iter() {
                if let Some(validated) = pending.validate() {
                    let (epoch, rinex) = validated.to_rinex(rtm.utc_time());

                    // redact message
                    match nav_tx.try_send(Message::Ephemeris((epoch, *sv, rinex))) {
                        Ok(_) => {},
                        Err(e) => {
                            error!(
                                "{}({}) failed to send collected ephemeris: {}",
                                epoch.round(cfg_precision),
                                sv,
                                e
                            );
                        },
                    }
                }
            }
        }

        if device.interface.is_read_only() {
            // In passive mode, there is not hardware acting as a throttle,
            // the channel capacity becomes the limit.
            // Adds a little bit of dead-time to reduce pressure on the data channel.
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}
