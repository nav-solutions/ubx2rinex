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

use std::str::FromStr;

use rinex::prelude::{Constellation, Duration, Epoch, Observable, TimeScale, SV};

use env_logger::{Builder, Target};
use log::{debug, error, info, trace, warn};

use tokio::sync::mpsc;

use ublox::{
    cfg_val::CfgVal, CfgLayer, CfgValSetBuilder, GpsFix, NavStatusFlags, NavStatusFlags2,
    NavTimeUtcFlags, PacketRef, RecStatFlags,
};

mod cli;
mod collecter;
mod device;
mod ubx;
mod utils;

use cli::Cli;
use collecter::{rawxm::Rawxm, Collecter, Message};
use device::Device;

use utils::to_constellation;

pub use ubx::Settings as UloxSettings;

async fn wait_sigterm(tx: mpsc::Sender<Message>) {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to wait for SHUTDOWN signal");
    let _ = tx.send(Message::Shutdown);
    panic!("terminated!");
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

    // cli
    let cli = Cli::new();

    // RINEX settings
    let settings = cli.rinex_settings();

    // init
    let mut buffer = [0; 8192];
    let mut uptime = Duration::default();

    let mut fix_type = GpsFix::NoFix; // current fix status
    let mut fix_flags = NavStatusFlags::empty(); // current fix flag
    let mut nav_status = NavStatusFlags2::Inactive;

    // UBlox settings
    let port = cli.port();
    let baud_rate = cli.baud_rate().unwrap_or(115_200);

    let mut ubx_settings = cli.ublox_settings();

    let (c1c, l1c, d1c) = if settings.major == 2 {
        (
            Observable::from_str("C1").unwrap(),
            Observable::from_str("L1").unwrap(),
            Observable::from_str("D1").unwrap(),
        )
    } else {
        (
            Observable::from_str("C1C").unwrap(),
            Observable::from_str("L1C").unwrap(),
            Observable::from_str("D1C").unwrap(),
        )
    };

    ubx_settings.observables.push(c1c);
    ubx_settings.observables.push(l1c);
    ubx_settings.observables.push(d1c);

    let (tx, rx) = mpsc::channel(32);
    let sigterm_tx = tx.clone();

    let mut collecter = Collecter::new(settings, ubx_settings.clone(), rx);

    tokio::spawn(async move {
        wait_sigterm(sigterm_tx).await;
    });

    tokio::spawn(async move {
        collecter.run().await;
    });

    // Open device
    let mut device = Device::open(port, baud_rate, &mut buffer);

    device.configure(&ubx_settings, &mut buffer, tx.clone());

    let now = Epoch::now().unwrap_or_else(|e| panic!("Failed to determine system time: {}", e));

    info!("{} - program deployed", now);

    loop {
        let _ = device.consume_all_cb(&mut buffer, |packet| {
            match packet {
                PacketRef::CfgNav5(pkt) => {
                    // Dynamic model
                    let _dyn_model = pkt.dyn_model();
                },
                PacketRef::RxmRawx(pkt) => {
                    let tow_nanos = (pkt.rcv_tow() * 1.0E9).round() as u64;
                    let week = pkt.week();

                    let t =
                        Epoch::from_time_of_week(week as u32, tow_nanos, ubx_settings.timescale);

                    let stat = pkt.rec_stat();

                    if stat.intersects(RecStatFlags::CLK_RESET) {
                        error!("{} - clock reset!", t);
                        warn!("{} - declaring phase cycle slip!", t);
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
                            debug!("unknown constellation: #{}", gnss_id);
                            continue;
                        }

                        let constell = constell.unwrap();

                        let prn = meas.sv_id();
                        let sv = SV::new(constell, prn);

                        let rawxm = Rawxm::new(t, sv, pr, cp, dop, cno);

                        match tx.try_send(Message::Measurement(rawxm)) {
                            Ok(_) => {
                                debug!("{}", rawxm);
                            },
                            Err(e) => {
                                error!("{}({}) missed measurement: {}", t, sv, e);
                            },
                        }
                    }
                },
                PacketRef::MonHw(_pkt) => {
                    //let jamming = pkt.jam_ind(); //TODO
                    //antenna problem:
                    // pkt.a_status();
                    //
                },
                PacketRef::NavSat(pkt) => {
                    debug!("nav-sat: {:?}", pkt);
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
                        let e = Epoch::maybe_from_gregorian(
                            pkt.year().into(),
                            pkt.month(),
                            pkt.day(),
                            pkt.hour(),
                            pkt.min(),
                            pkt.sec(),
                            pkt.nanos() as u32,
                            TimeScale::UTC,
                        );
                    }
                },
                PacketRef::NavStatus(pkt) => {
                    //itow = pkt.itow();
                    fix_type = pkt.fix_type();
                    fix_flags = pkt.flags();
                    nav_status = pkt.flags2();
                    uptime = Duration::from_milliseconds(pkt.uptime_ms() as f64);
                    trace!("uptime: {}", uptime);
                },
                PacketRef::NavEoe(pkt) => {
                    let itow = pkt.itow();
                    // reset Epoch
                    // lli = None;
                    // epoch_flag = EpochFlag::default();
                    debug!("EOE | itow = {}", itow);
                },
                PacketRef::NavPvt(pkt) => {
                    debug!("NAV PVT: {:?}", pkt);
                },
                PacketRef::MgaGpsEph(pkt) => {
                    // let _sv = sv!(&format!("G{}", pkt.sv_id()));
                    //nav_record.insert(epoch, sv);
                },
                PacketRef::MgaGloEph(pkt) => {
                    // let _sv = sv!(&format!("R{}", pkt.sv_id()));
                    //nav_record.insert(epoch, sv);
                },
                /*
                 * NAVIGATION: IONOSPHERIC MODELS
                 */
                PacketRef::MgaGpsIono(pkt) => {
                    // let kbmodel = KbModel {
                    //     alpha: (pkt.alpha0(), pkt.alpha1(), pkt.alpha2(), pkt.alpha3()),
                    //     beta: (pkt.beta0(), pkt.beta1(), pkt.beta2(), pkt.beta3()),
                    //     region: KbRegionCode::default(), // TODO,
                    // };
                    // let _iono = IonMessage::KlobucharModel(kbmodel);
                },
                PacketRef::NavClock(pkt) => {
                    let clock = pkt.clk_bias();
                    match tx.try_send(Message::Clock(clock)) {
                        Ok(_) => {
                            debug!("{}", clock);
                        },
                        Err(e) => {
                            error!("missed clock state: {}", e);
                        },
                    }
                },
                /*
                 * Errors, Warnings
                 */
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
                pkt => {
                    warn!("main: {:?}", pkt);
                }, // unused
            }
        });
    } // loop
}
