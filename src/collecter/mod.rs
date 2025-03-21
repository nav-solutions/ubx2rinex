use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

use rinex::{
    observation::ClockObservation,
    observation::HeaderFields as ObsHeader,
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Duration, Epoch, Header, Observable, CRINEX,
    },
};

mod fd;

pub mod observation;
pub mod rawxm;
pub mod settings;

use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
};

use log::error;

use crate::{
    collecter::{
        fd::FileDescriptor, observation::Collecter as ObsCollecter, rawxm::Rawxm,
        settings::Settings,
    },
    UbloxSettings,
};

#[derive(Debug, Clone)]
pub enum Message {
    /// [Message::Shutdown] catches Ctrl+C interruptions
    Shutdown,
    /// [Message::EndofEpoch] notification
    EndofEpoch,
    /// Timestamp / [Epoch] update
    Timestamp(Epoch),
    /// New clock state [s]
    Clock(f64),
    /// New [Rawxm] measurements
    Measurement(Rawxm),
    /// Firmware version notification
    FirmwareVersion(String),
}

pub struct Collecter {
    t: Option<Epoch>,
    t0: Option<Epoch>,
    rx: Receiver<Message>,
    shutdown: bool,
    settings: Settings,
    ubx_settings: UbloxSettings,
}

impl Collecter {
    /// Builds new [Collecter]
    pub fn new(settings: Settings, ublox: UbloxSettings, rx: Receiver<Message>) -> Self {
        Self {
            rx,
            t0: None,
            t: None,
            settings,
            shutdown: false,
            ubx_settings: ublox,
        }
    }

    /// Obtain a new file descriptor
    fn fd(&self, t: Epoch) -> FileDescriptor {
        let filename = self.settings.filename(t);
        FileDescriptor::new(self.settings.gzip, &filename)
    }

    pub async fn run(&mut self) {
        let (obs_tx, obs_rx) = mpsc::channel(8);
        let (nav_tx, nav_rx) = mpsc::channel(8);

        if self.settings.obs_rinex {
            let mut tasklet =
                ObsCollecter::new(self.settings.clone(), self.ubx_settings.clone(), obs_rx);

            tokio::spawn(async move {
                tasklet.run().await;
            });
        }

        loop {
            match self.rx.try_recv() {
                Ok(msg) => {
                    let _ = obs_tx.send(msg.clone());
                    let _ = nav_tx.send(msg);
                },
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                },
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// Returns collecter uptime (whole session)
    fn uptime(&self, t: Epoch) -> Option<Duration> {
        let t0 = self.t0?;
        Some(t - t0)
    }
}
