use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

use tokio::sync::mpsc::Receiver;

use log::error;

use rinex::{
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Header, Observable,
        Duration, Epoch,
    },
};

use crate::UbloxSettings;

mod fd;

pub mod rawxm;
pub mod settings;

use fd::FileDescriptor;
use rawxm::Rawxm;
use settings::Settings;

#[derive(Debug)]
pub enum Message {
}

pub struct NavCollecter {
    t: Option<Epoch>,
    t0: Option<Epoch>,
    buf: Observations,
    rx: Receiver<Message>,
    settings: Settings,
    fd: Option<BufWriter<FileDescriptor>>,
}

impl NavCollecter {
    /// Builds new [NavCollecter]
    pub fn new(settings: Settings, ublox: UbloxSettings, rx: Receiver<Message>) -> Self {
        Self {
            rx,
            fd: None,
            t0: None,
            t: None,
            settings,
            shutdown: false,
            obs_header: None,
            ubx_settings: ublox,
            buf: Observations::default(),
        }
    }

    /// Obtain a new file descriptor
    fn fd(&self, t: Epoch) -> FileDescriptor {
        let filename = self.settings.filename(t);
        FileDescriptor::new(self.settings.gzip, &filename)
    }

    pub async fn run(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(msg) => match msg {
                    Message::FirmwareVersion(version) => {
                        self.ubx_settings.firmware = Some(version.to_string());
                    },

                    Message::Shutdown => {
                        return;
                    },
                },
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                },
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    fn release_header(&mut self) {
        let t0 = self.t0.unwrap();

        // obtain new file, release header
        let mut fd = BufWriter::new(self.fd(t0));

        let header = self.build_header();

        header.format(&mut fd).unwrap_or_else(|e| {
            panic!(
                "Observation header formatting: {}. Aborting (avoiding corrupt file)",
                e
            )
        });

        let _ = fd.flush();

        self.fd = Some(fd);
        self.obs_header = Some(header.obs.unwrap().clone());
    }

    fn release_epoch(&mut self) {
        let t = self.t.unwrap();

        let key = ObsKey {
            epoch: t,
            flag: EpochFlag::Ok, // TODO,
        };

        let mut fd = self.fd.as_mut().unwrap();

        let obs_header = self
            .obs_header
            .as_ref()
            .expect("internal error: missing Observation header");

        match self
            .buf
            .format(self.settings.major == 2, &key, obs_header, &mut fd)
        {
            Ok(_) => {
                let _ = fd.flush();
                self.buf.clock = None;
                self.buf.signals.clear();
            },
            Err(e) => {
                error!("{} formatting issue: {}", t, e);
            },
        }
    }

    fn build_header(&self) -> Header {
        let mut header = Header::default();
        let mut obs_header = ObsHeader::default();

        header.version.major = self.settings.major;

        if self.settings.crinex {
            let mut crinex = CRINEX::default();

            if self.settings.major == 2 {
                crinex.version.major = 2;
            } else {
                crinex.version.major = 3;
            }

            obs_header.crinex = Some(crinex);
        }

        if let Some(operator) = &self.settings.operator {
            header.observer = Some(operator.clone());
        }

        if let Some(agency) = &self.settings.agency {
            header.agency = Some(agency.clone());
        }

        for constellation in self.ubx_settings.constellations.iter() {
            for observable in self.ubx_settings.observables.iter() {
                if let Some(codes) = obs_header.codes.get_mut(constellation) {
                    codes.push(observable.clone());
                } else {
                    obs_header
                        .codes
                        .insert(*constellation, vec![observable.clone()]);
                }
            }
        }

        header.obs = Some(obs_header);
        header
    }

    /// Returns collecter uptime (whole session)
    fn uptime(&self, t: Epoch) -> Option<Duration> {
        let t0 = self.t0?;
        Some(t - t0)
    }
}
