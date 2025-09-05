use log::{debug, error, trace};

use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

use rinex::{
    error::FormattingError,
    hardware::{Antenna, Receiver},
    observation::{ClockObservation, HeaderFields as ObsHeader},
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Constellation, Epoch, Header, Observable, RinexType, CRINEX,
    },
};

use tokio::{sync::mpsc::Receiver as Rx, sync::watch::Receiver as WatchRx};

use crate::{
    collecter::{fd::FileDescriptor, settings::Settings, Message},
    UbloxSettings,
};

use hifitime::prelude::Duration;

pub struct Collecter {
    /// Latest [Epoch]
    epoch: Option<Epoch>,

    /// [Epoch] of deployment
    deploy_epoch: Option<Epoch>,

    /// Current [Observations] buffer
    buf: Observations,

    /// Redacted [ObsHeader]
    header: Option<ObsHeader>,

    /// [Message]ing handle
    rx: Rx<Message>,

    /// graceful exit
    shutdown: WatchRx<bool>,

    /// [Settings]
    settings: Settings,

    /// [UbloxSettings]
    ubx_settings: UbloxSettings,

    /// Current [FileDescriptor] handle
    fd: Option<BufWriter<FileDescriptor>>,

    /// List of header comments
    header_comments: Vec<String>,
}

impl Collecter {
    /// Builds new [Collecter]
    pub fn new(
        settings: Settings,
        ublox: UbloxSettings,
        shutdown: WatchRx<bool>,
        rx: Rx<Message>,
    ) -> Self {
        Self {
            rx,
            shutdown,
            settings,
            ubx_settings: ublox,
            fd: Default::default(),
            deploy_epoch: Default::default(),
            epoch: Default::default(),
            header: Default::default(),
            buf: Observations::default(),
            header_comments: Default::default(),
        }
    }

    /// Obtain a new file descriptor
    fn fd(&self, t: Epoch) -> FileDescriptor {
        let filename = self.settings.filename(false, t);
        FileDescriptor::new(self.settings.gzip, &filename)
    }

    pub async fn run(&mut self) {
        let cfg_precision = Duration::from_seconds(1.0);

        // TODO: improve observables definition & handling..
        let c1c = if self.settings.major == 3 {
            Observable::from_str("C1C").unwrap()
        } else {
            Observable::from_str("C1").unwrap()
        };

        let l1c = if self.settings.major == 3 {
            Observable::from_str("L1C").unwrap()
        } else {
            Observable::from_str("L1").unwrap()
        };

        let d1c = if self.settings.major == 3 {
            Observable::from_str("D1C").unwrap()
        } else {
            Observable::from_str("D1").unwrap()
        };

        loop {
            match self.rx.recv().await {
                Some(msg) => match msg {
                    Message::FirmwareVersion(version) => {
                        self.ubx_settings.firmware = Some(version.to_string());
                    },

                    Message::Shutdown => {
                        if self.buf.signals.len() > 0 || self.buf.clock.is_some() {
                            self.release_epoch();
                        }

                        return; // abort
                    },

                    Message::HeaderComment(comment) => {
                        if self.header_comments.len() < 16 {
                            self.header_comments.push(comment);
                        }
                    },

                    Message::Clock(clock) => {
                        debug!(
                            "{} - new clock state: {}",
                            self.epoch.unwrap_or_default().round(cfg_precision),
                            Duration::from_seconds(clock)
                        );

                        let bias = clock * 1.0E-3;
                        let mut clock = ClockObservation::default();
                        clock.set_offset_s(Default::default(), bias);
                        self.buf.clock = Some(clock);
                    },

                    Message::Measurement(rawxm) => {
                        debug!(
                            "{} - RXM-RAWX: {}",
                            self.epoch.unwrap_or_default().round(cfg_precision),
                            rawxm.epoch
                        );

                        if self.deploy_epoch.is_none() {
                            self.deploy_epoch = Some(rawxm.epoch);
                            match self.release_header() {
                                Ok(_) => {
                                    debug!(
                                        "{} - RINEX header redacted",
                                        self.epoch.unwrap_or_default().round(cfg_precision)
                                    );
                                },
                                Err(e) => {
                                    error!(
                                        "{} - failed to redact RINEX header: {}",
                                        self.epoch.unwrap_or_default().round(cfg_precision),
                                        e
                                    );
                                    return;
                                },
                            }
                        }

                        if self.epoch.is_none() {
                            self.epoch = Some(rawxm.epoch);
                        }

                        let epoch = self.epoch.unwrap();

                        if rawxm.epoch > epoch {
                            // new epoch
                            debug!("{} - new epoch", rawxm.epoch.round(cfg_precision));

                            if self.buf.signals.len() > 0 || self.buf.clock.is_some() {
                                self.release_epoch();
                            }
                        }

                        self.buf.signals.push(SignalObservation {
                            sv: rawxm.sv,
                            lli: None,
                            snr: None,
                            value: rawxm.cp,
                            observable: c1c.clone(),
                        });

                        self.buf.signals.push(SignalObservation {
                            sv: rawxm.sv,
                            lli: None,
                            snr: None,
                            value: rawxm.pr,
                            observable: l1c.clone(),
                        });

                        self.buf.signals.push(SignalObservation {
                            sv: rawxm.sv,
                            lli: None,
                            snr: None,
                            value: rawxm.dop as f64,
                            observable: d1c.clone(),
                        });

                        self.epoch = Some(rawxm.epoch);
                    },
                    _ => {},
                },
                None => {},
            }
        }
    }

    fn release_header(&mut self) -> Result<(), FormattingError> {
        let deploy_epoch = self.deploy_epoch.unwrap();

        // obtain new file, release header
        let mut fd = BufWriter::new(self.fd(deploy_epoch));

        let header = self.build_header();

        header.format(&mut fd)?; // must pass

        let _ = fd.flush(); // can fail

        self.fd = Some(fd);
        self.header = Some(header.obs.unwrap().clone());

        Ok(())
    }

    fn release_epoch(&mut self) {
        let epoch = self.epoch.unwrap_or_default();

        let key = ObsKey {
            epoch,
            flag: EpochFlag::Ok, // TODO: manage events correctly
        };

        let mut fd = self.fd.as_mut().unwrap();

        match self.header.as_ref() {
            Some(header) => {
                match self
                    .buf
                    .format(self.settings.major == 2, &key, header, &mut fd)
                {
                    Ok(_) => {
                        let _ = fd.flush(); // improves interaction

                        self.buf.clock = None;
                        self.buf.signals.clear();

                        debug!("{} - new epoch released", epoch);
                    },
                    Err(e) => {
                        error!("{} - failed to format pending epoch: {}", epoch, e);
                    },
                }
            },
            None => {
                error!(
                    "{} - internal error: failed to release pending epoch",
                    epoch
                );

                error!("{} - internal error: incomplete RINEX header", epoch);
            },
        }
    }

    fn build_header(&self) -> Header {
        let mut header = Header::default();

        let mut antenna = Option::<Antenna>::None;
        let mut receiver = Option::<Receiver>::None;

        let mut obs_header = ObsHeader::default();

        // revision
        header.rinex_type = RinexType::ObservationData;
        header.version.major = self.settings.major;

        // GNSS
        if self.ubx_settings.constellations.len() == 1 {
            header.constellation = Some(self.ubx_settings.constellations[0]);
        } else {
            header.constellation = Some(Constellation::Mixed);
        }

        // CRINEX
        if self.settings.crinex {
            let mut crinex = CRINEX::default();

            if self.settings.major == 2 {
                crinex.version.major = 2;
            } else {
                crinex.version.major = 3;
            }

            obs_header.crinex = Some(crinex);
        }

        // real time flow comments
        for comment in self.header_comments.iter() {
            header.comments.push(comment.to_string());
        }

        // user comment
        if let Some(comment) = &self.settings.header_comment {
            header.comments.push(comment.to_string());
        }

        // custom operator
        if let Some(operator) = &self.settings.operator {
            header.observer = Some(operator.clone());
        }

        // custom agency
        if let Some(agency) = &self.settings.agency {
            header.agency = Some(agency.clone());
        }

        // custom receiver
        if let Some(model) = &self.ubx_settings.model {
            if let Some(receiver) = &mut receiver {
                *receiver = receiver.with_model(model);
            } else {
                receiver = Some(Receiver::default().with_model(model));
            }
        }

        if let Some(firmware) = &self.ubx_settings.firmware {
            if let Some(receiver) = &mut receiver {
                *receiver = receiver.with_firmware(firmware);
            } else {
                receiver = Some(Receiver::default().with_firmware(firmware));
            }
        }

        header.rcvr = receiver;

        // custom antenna
        if let Some(model) = &self.ubx_settings.antenna {
            antenna = Some(Antenna::default().with_model(model));
        }

        header.rcvr_antenna = antenna;

        obs_header.codes = self.settings.observables.clone();

        header.obs = Some(obs_header);
        header
    }
}
