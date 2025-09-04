#![doc(html_logo_url = "https://raw.githubusercontent.com/rtk-rs/.github/master/logos/logo2.jpg")]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::type_complexity)]

/*
 * UBX2RINEX is part of the rtk-rs framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al,
 * (cf. https://github.com/rtk-rs/rinex/graphs/contributors)
 * (cf. https://github.com/rtk-rs/ubx2rinex/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation: https://github.com/rtk-rs/ubx2rinex
 */

extern crate gnss_rs as gnss;
extern crate ublox;

use env_logger::{Builder, Target};

use log::{debug, error, info, trace, warn};

use tokio::{
    signal,
    sync::{mpsc, watch},
};

use std::fs::File;

use rinex::prelude::{Constellation, Duration, Epoch, TimeScale, SV};

use ublox::{NavTimeUtcFlags, PacketRef, RecStatFlags};

mod cli;
mod collecter;
mod device;
mod runtime;
mod ubx;
mod utils;

use crate::{
    cli::Cli,
    collecter::{
        navigation::Collecter as NavCollecter, observation::Collecter as ObsCollecter,
        rawxm::Rawxm, Message,
    },
    device::Device,
    runtime::Runtime,
    ubx::Settings as UbloxSettings,
    utils::to_constellation,
};

fn consume_device(
    runtime: &mut Runtime,
    obs_tx: &mut mpsc::Sender<Message>,
    nav_tx: &mut mpsc::Sender<Message>,
    device: &mut Device,
    buffer: &mut [u8],
    cfg_timescale: TimeScale,
    cfg_precision: Duration,
) -> std::io::Result<usize> {
    let mut end_of_nav_epoch = false;

    device.consume_all_cb(buffer, |packet| {
        match packet {
            PacketRef::CfgNav5(_) => {
                // TODO: Dynamic model ?
                // let _dyn_model = pkt.dyn_model();
            },
            PacketRef::RxmSfrbx(sfrbx) => {
                let gnss_id = sfrbx.gnss_id();

                match to_constellation(gnss_id) {
                    Some(constellation) => {
                        let sv = SV::new(constellation, sfrbx.sv_id());

                        match constellation {
                            Constellation::GPS | Constellation::QZSS => {
                                // decode
                                if let Some(interpretation) = sfrbx.interpret() {
                                    runtime.latch_sfrbx(sv, interpretation, cfg_precision);
                                } else {
                                    debug!(
                                        "{} - SFRBX interpretation issue",
                                        runtime.utc_time().round(cfg_precision)
                                    );
                                }
                            },
                            c => {
                                trace!(
                                    "{} - {} constellation not handled yet",
                                    runtime.utc_time().round(cfg_precision),
                                    c
                                );
                            },
                        }
                    },
                    None => {
                        debug!(
                            "{} - constellation id error #{}",
                            runtime.utc_time().round(cfg_precision),
                            gnss_id
                        );
                    },
                }
            },
            PacketRef::RxmRawx(pkt) => {
                let gpst_tow_nanos = (pkt.rcv_tow() * 1.0E9).round() as u64;
                let t_gpst =
                    Epoch::from_time_of_week(pkt.week() as u32, gpst_tow_nanos, TimeScale::GPST);

                runtime.new_epoch(t_gpst, cfg_timescale);

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
                    let _pr_stddev = meas.pr_stdev();

                    let cp = meas.cp_mes();
                    let _cp_stddev = meas.cp_stdev();

                    let dop = meas.do_mes();
                    let _dop_stddev = meas.do_stdev();

                    // let freq_id = meas.freq_id();
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

                    let prn = meas.sv_id();
                    let sv = SV::new(constell, prn);
                    let t_meas = t_gpst.to_time_scale(cfg_timescale);

                    let rawxm = Rawxm::new(t_meas, sv, pr, cp, dop, cno);

                    match obs_tx.try_send(Message::Measurement(rawxm)) {
                        Ok(_) => {
                            debug!("{}({}) - RAWXM {}", t_meas.round(cfg_precision), sv, rawxm);
                        },
                        Err(e) => {
                            error!(
                                "{}({}) missed measurement: {}",
                                t_meas.round(cfg_precision),
                                sv,
                                e
                            );
                        },
                    }
                }
            },
            PacketRef::MonHw(_pkt) => {
                // TODO customize RINEX header
            },
            PacketRef::NavSat(pkt) => {
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

                    let _sv = SV {
                        constellation,
                        prn: sv.sv_id(),
                    };

                    // flags.sv_used()
                    //flags.health();
                    //flags.quality_ind();
                    //flags.differential_correction_available();
                    //flags.ephemeris_available();
                }
            },
            PacketRef::NavTimeUTC(pkt) => {
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
            PacketRef::NavStatus(pkt) => {
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
            PacketRef::NavEoe(pkt) => {
                let gpst_itow_nanos = pkt.itow() as u64 * 1_000_000;

                let t_gpst =
                    Epoch::from_time_of_week(runtime.gpst_week(), gpst_itow_nanos, TimeScale::GPST);

                end_of_nav_epoch = true;

                debug!("{} - End of Epoch", t_gpst.round(cfg_precision));

                let _ = nav_tx.try_send(Message::EndofEpoch(t_gpst));
            },

            PacketRef::NavPvt(pkt) => {
                let (y, m, d) = (pkt.year() as i32, pkt.month(), pkt.day());
                let (hh, mm, ss) = (pkt.hour(), pkt.min(), pkt.sec());
                if pkt.valid() > 2 {
                    let t_solution = Epoch::from_gregorian(y, m, d, hh, mm, ss, 0, TimeScale::UTC)
                        .to_time_scale(cfg_timescale);

                    info!(
                        "{} - PVT SOLUTION: lat={:.5E}° long={:.5E}°",
                        t_solution.round(cfg_precision),
                        pkt.latitude(),
                        pkt.longitude()
                    );
                }
            },

            PacketRef::MgaGpsEph(_) | PacketRef::MgaGloEph(_) => {
                // MGA-EPH downlink
            },

            PacketRef::MgaGpsIono(_) => {
                // MGA-IONO downlink
            },

            PacketRef::NavClock(pkt) => {
                let clock = pkt.clk_bias();
                match obs_tx.try_send(Message::Clock(clock)) {
                    Ok(_) => {
                        debug!("{}", clock);
                    },
                    Err(e) => {
                        error!("missed clock state: {}", e);
                    },
                }
            },

            PacketRef::InfTest(pkt) => {
                if let Some(msg) = pkt.message() {
                    trace!("{}", msg);
                }
            },
            PacketRef::InfDebug(pkt) => {
                if let Some(msg) = pkt.message() {
                    debug!("{}", msg);
                }
            },
            PacketRef::InfNotice(pkt) => {
                if let Some(msg) = pkt.message() {
                    info!("{}", msg);
                }
            },
            PacketRef::InfError(pkt) => {
                if let Some(msg) = pkt.message() {
                    error!("{}", msg);
                }
            },
            PacketRef::InfWarning(pkt) => {
                if let Some(msg) = pkt.message() {
                    warn!("{}", msg);
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
        Device::open_serial_port(serial, baud_rate, &mut buffer)
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
    let (mut obs_tx, obs_rx) = mpsc::channel(32);

    let mut obs_collecter = ObsCollecter::new(
        settings.clone(),
        ubx_settings.clone(),
        shutdown_rx.clone(),
        obs_rx,
    );

    // Navigation RINEX
    let (mut nav_tx, nav_rx) = mpsc::channel(32);

    let mut nav_collecter = NavCollecter::new(
        t_utc,
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
    let mut rtm = Runtime::new(t_utc);
    info!("{} - application deployed", t_utc.round(cfg_precision));

    loop {
        match consume_device(
            &mut rtm,
            &mut obs_tx,
            &mut nav_tx,
            &mut device,
            &mut buffer,
            ubx_settings.timescale,
            cfg_precision,
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
                    let (epoch, rinex) = validated.to_rinex();

                    // redact message
                    let _ = nav_tx.send(Message::Ephemeris((epoch, *sv, rinex)));
                }
            }
        }
    }
}
