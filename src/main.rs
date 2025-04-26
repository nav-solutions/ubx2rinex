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

use env_logger::{Builder, Target};

use log::{debug, error, info, trace, warn};

use tokio::{
    signal,
    sync::{mpsc, watch},
};

use rinex::prelude::{Constellation, Duration, Epoch, Observable, TimeScale, SV};

use ublox::{GpsFix, NavStatusFlags, NavStatusFlags2, NavTimeUtcFlags, PacketRef, RecStatFlags};

mod cli;
mod collecter;
mod device;
mod ubx;
mod utils;

use crate::{
    cli::Cli,
    collecter::{
        ephemeris::EphemerisBuilder, navigation::Collecter as NavCollecter,
        observation::Collecter as ObsCollecter, rawxm::Rawxm, Message,
    },
    device::Device,
    ubx::Settings as UbloxSettings,
    utils::to_constellation,
};

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

    let timescale = ubx_settings.timescale;

    // Time
    let mut t_utc = Epoch::now()
        .unwrap_or_else(|e| panic!("Failed to determine system time: {}", e))
        .to_time_scale(TimeScale::UTC);

    let mut nav_utc_week = t_utc.to_time_of_week().0;

    let mut t_gpst = t_utc.to_time_scale(TimeScale::GPST);

    let mut nav_gpst = t_gpst;
    let mut nav_gpst_week = t_gpst.to_time_of_week().0;

    let mut t_gst = t_utc.to_time_scale(TimeScale::GST);

    let mut nav_gst = t_gst;
    let mut nav_gst_week = t_gst.to_time_of_week().0;

    let mut t_bdt = t_utc.to_time_scale(TimeScale::BDT);

    let mut nav_bdt = t_bdt;
    let mut nav_bdt_week = t_bdt.to_time_of_week().0;

    let mut end_of_nav_epoch = false;

    // Tokio
    let (shutdown_tx, shutdown_rx) = watch::channel(true);

    // Observation RINEX
    let (obs_tx, obs_rx) = mpsc::channel(32);
    let mut obs_collecter = ObsCollecter::new(
        settings.clone(),
        ubx_settings.clone(),
        shutdown_rx.clone(),
        obs_rx,
    );

    // Navigation RINEX
    let (nav_tx, nav_rx) = mpsc::channel(32);
    let mut nav_collecter = NavCollecter::new(
        t_utc,
        settings.clone(),
        ubx_settings.clone(),
        shutdown_rx.clone(),
        nav_rx,
    );

    // Open device
    let mut device = Device::open(port, baud_rate, &mut buffer);

    device.configure(&ubx_settings, &mut buffer, obs_tx.clone());

    if ubx_settings.rawxm {
        tokio::spawn(async move {
            debug!("{} - Observation mode deployed", t_utc);
            obs_collecter.run().await;
        });
    }

    if ubx_settings.ephemeris {
        tokio::spawn(async move {
            debug!("{} - Navigation  mode deployed", t_utc);
            nav_collecter.run().await;
        });
    }

    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .unwrap_or_else(|e| panic!("Tokio signal handling error: {}", e));

        shutdown_tx
            .send(true)
            .unwrap_or_else(|e| panic!("Tokio: signaling error: {}", e));
    });

    loop {
        let _ = device.consume_all_cb(&mut buffer, |packet| {
            match packet {
                PacketRef::CfgNav5(pkt) => {
                    // Dynamic model
                    let _dyn_model = pkt.dyn_model();
                },
                PacketRef::RxmRawx(pkt) => {
                    let gpst_tow_nanos = (pkt.rcv_tow() * 1.0E9).round() as u64;
                    t_gpst = Epoch::from_time_of_week(pkt.week() as u32, gpst_tow_nanos, timescale);

                    let stat = pkt.rec_stat();

                    if stat.intersects(RecStatFlags::CLK_RESET) {
                        error!("{} - clock reset!", t_gpst);
                        warn!("{} - declaring phase cycle slip!", t_gpst);
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

                        let t = if settings.timescale == TimeScale::GPST {
                            t_gpst
                        } else {
                            t_gpst.to_time_scale(settings.timescale)
                        };

                        let rawxm = Rawxm::new(t, sv, pr, cp, dop, cno);

                        match obs_tx.try_send(Message::Measurement(rawxm)) {
                            Ok(_) => {
                                debug!("{}", rawxm);
                            },
                            Err(e) => {
                                error!("{}({}) missed measurement: {}", t_gpst, sv, e);
                            },
                        }
                    }
                },
                PacketRef::MonHw(pkt) => {},
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
                    pkt.itow();
                    //itow = pkt.itow();
                    uptime = Duration::from_milliseconds(pkt.uptime_ms() as f64);
                    trace!(
                        "Fix status: {:?} | {:?} | {:?}",
                        pkt.fix_stat(),
                        pkt.flags(),
                        pkt.flags2()
                    );
                    trace!("Uptime: {}", uptime);
                },
                PacketRef::NavEoe(pkt) => {
                    let nav_gpst_itow_nanos = pkt.itow() as u64 * 1_000_000;

                    nav_gpst = Epoch::from_time_of_week(
                        nav_gpst_week,
                        nav_gpst_itow_nanos,
                        TimeScale::GPST,
                    );

                    end_of_nav_epoch = true;

                    debug!("{} - End of Epoch", nav_gpst);
                    let _ = nav_tx.try_send(Message::EndofEpoch(nav_gpst));
                },
                PacketRef::NavPvt(pkt) => {
                    let (y, m, d) = (pkt.year() as i32, pkt.month(), pkt.day());
                    let (hh, mm, ss) = (pkt.hour(), pkt.min(), pkt.sec());
                    if pkt.valid() > 2 {
                        t_utc = Epoch::from_gregorian(y, m, d, hh, mm, ss, 0, TimeScale::UTC)
                            .to_time_scale(timescale);

                        info!(
                            "{} - nav-pvt: lat={:.5E}° long={:.5E}°",
                            t_utc,
                            pkt.latitude(),
                            pkt.longitude()
                        );
                    }
                },
                PacketRef::MgaGpsEph(pkt) => {
                    debug!("{:?}", pkt);
                    let sv = SV::new(Constellation::GPS, pkt.sv_id());
                    let eph = EphemerisBuilder::from_gps(pkt);

                    match nav_tx.try_send(Message::Ephemeris((t_gpst, sv, eph))) {
                        Ok(_) => {},
                        Err(e) => {
                            error!("missed GPS ephemeris: {}", e);
                        },
                    }
                },
                PacketRef::MgaGloEph(pkt) => {
                    debug!("{:?}", pkt);
                    let sv = SV::new(Constellation::GPS, pkt.sv_id());
                    let eph = EphemerisBuilder::from_glonass(pkt);

                    match nav_tx.try_send(Message::Ephemeris((t_utc, sv, eph))) {
                        Ok(_) => {},
                        Err(e) => {
                            error!("missed Glonass ephemeris: {}", e);
                        },
                    }
                },
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
            }
        });

        if end_of_nav_epoch {
            if ubx_settings.constellations.contains(&Constellation::GPS) {
                device.request_mga_gps_eph();
            }

            if ubx_settings
                .constellations
                .contains(&Constellation::Glonass)
            {
                device.request_mga_glonass_eph();
            }

            end_of_nav_epoch = false;
        }
    } // loop
}
