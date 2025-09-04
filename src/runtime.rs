use hifitime::prelude::{Duration, Epoch, TimeScale};
use log::trace;

use ublox::{
    // NavStatusFlags,
    // NavStatusFlags2,
    RxmSfrbxInterpreted,
};

use rinex::prelude::{Constellation, SV};

use crate::collecter::ephemeris::{PendingFrame, PendingGpsQzssFrame};

use std::collections::HashMap;

#[derive(Debug)]
pub struct Runtime {
    /// Current [Epoch]
    pub epoch: Epoch,

    /// Epoch of deployment
    deploy_time: Epoch,

    /// Uptime as [Duration]
    pub uptime: Duration,

    // /// Current fix flag
    // pub fix_flag: NavStatusFlags,
    // /// Current nav status
    // pub nav_status: NavStatusFlags2,

    /// [PendingFrame]s
    pub pending_frames: HashMap<SV, PendingFrame>,
}

impl Runtime {
    pub fn new(epoch: Epoch) -> Self {
        Self {
            epoch,
            deploy_time: epoch,
            uptime: Default::default(),
            // fix_flag: NavStatusFlags::empty(),
            // nav_status: NavStatusFlags2::Inactive,
            pending_frames: Default::default(),
        }
    }

    /// Update latest epoch
    pub fn new_epoch(&mut self, epoch: Epoch, cfg_timescale: TimeScale) {
        self.epoch = epoch.to_time_scale(cfg_timescale);
        self.uptime = epoch - self.deploy_time;
    }

    /// Latch new SFRBX interpretation
    pub fn latch_sfrbx(
        &mut self,
        sv: SV,
        interpretation: RxmSfrbxInterpreted,
        cfg_precision: Duration,
    ) {
        if let Some(pending) = &mut self.pending_frames.get_mut(&sv) {
            pending.update(interpretation);
        } else {
            match (sv.constellation, interpretation) {
                (Constellation::GPS | Constellation::QZSS, RxmSfrbxInterpreted::GpsQzss(frame)) => {
                    self.pending_frames
                        .insert(sv, PendingFrame::GpsQzss(PendingGpsQzssFrame::new(frame)));
                },
                (c, _) => trace!(
                    "{} - {} constellation not supported yet",
                    self.utc_time().round(cfg_precision),
                    c
                ),
            }
        }
    }

    // /// Tries to gather a [GpsQzssEphemeris]
    // pub fn gather_gps_qzss_ephemeris(&self) -> Option<GpsQzssEphemeris> {
    //     let pending = self.pending_gps_qzss_frame?;
    //     pending.validate()
    // }

    /// Returns current epoch
    pub fn current_epoch(&self, timescale: TimeScale) -> Epoch {
        self.epoch.to_time_scale(timescale)
    }

    /// Returns current week number in desired [TimeScale]
    pub fn current_week(&self, timescale: TimeScale) -> u32 {
        self.epoch.to_time_scale(timescale).to_time_of_week().0
    }

    // /// Returns current epoch in [TimeScale::GPST]
    // pub fn gpst_time(&self) -> Epoch {
    //     self.current_epoch(TimeScale::GPST)
    // }

    /// Returns current epoch in [TimeScale::UTC]
    pub fn utc_time(&self) -> Epoch {
        self.current_epoch(TimeScale::UTC)
    }

    /// Returns current [TimeScale::GPST] week
    pub fn gpst_week(&self) -> u32 {
        self.current_week(TimeScale::GPST)
    }
}
