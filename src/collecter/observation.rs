use std::{
    io::{BufWriter, Write},
    str::FromStr,
};

use rinex::{
    observation::{ClockObservation, HeaderFields as ObsHeader},
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Epoch, Header, Observable, CRINEX,
    },
};

use tokio::{
    sync::mpsc::Receiver as Rx,
    sync::watch::Receiver as WatchRx,
    time::{sleep, Duration},
};

use log::error;

use crate::{
    collecter::{fd::FileDescriptor, rawxm::Rawxm, settings::Settings, Message},
    UbloxSettings,
};

pub struct Collecter {
    t: Option<Epoch>,
    t0: Option<Epoch>,
    buf: Observations,
    header: Option<ObsHeader>,
    rx: Rx<Message>,
    shutdown: WatchRx<bool>,
    settings: Settings,
    ubx_settings: UbloxSettings,
    fd: Option<BufWriter<FileDescriptor>>,
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
            fd: None,
            t0: None,
            t: None,
            header: None,
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
            tokio::select! {
                _ = self.shutdown.changed() => {
                    println!("CAUGHT CHANGES");
                    // Stop current work
                    return ;
                },
                _ = sleep(Duration::from_millis(10)) => {
                    self.tasklet().await;
                },
            }
        }
    }

    pub async fn tasklet(&mut self) {
        match self.rx.recv().await {
            Some(msg) => match msg {
                Message::EndofEpoch => {},
                Message::Timestamp(t) => {},
                Message::FirmwareVersion(version) => {
                    self.ubx_settings.firmware = Some(version.to_string());
                },

                Message::Shutdown => {
                    if self.buf.signals.len() > 0 || self.buf.clock.is_some() {
                        self.release_epoch();
                    }
                    return;
                },

                Message::Clock(clock) => {
                    let bias = clock * 1.0E-3;
                    let mut clock = ClockObservation::default();
                    clock.set_offset_s(Default::default(), bias);
                    self.buf.clock = Some(clock);
                },

                Message::Measurement(rawxm) => {
                    if self.t0.is_none() {
                        self.t0 = Some(rawxm.t);
                        self.release_header();
                    }

                    if self.t.is_none() {
                        self.t = Some(rawxm.t);
                    }

                    let t = self.t.unwrap();

                    if rawxm.t > t {
                        if self.buf.signals.len() > 0 || self.buf.clock.is_some() {
                            self.release_epoch();
                        }
                    }

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

                    self.buf.signals.push(SignalObservation {
                        sv: rawxm.sv,
                        lli: None,
                        snr: None,
                        value: rawxm.cp,
                        observable: c1c,
                    });

                    self.buf.signals.push(SignalObservation {
                        sv: rawxm.sv,
                        lli: None,
                        snr: None,
                        value: rawxm.pr,
                        observable: l1c,
                    });

                    self.buf.signals.push(SignalObservation {
                        sv: rawxm.sv,
                        lli: None,
                        snr: None,
                        value: rawxm.dop as f64,
                        observable: d1c,
                    });

                    self.t = Some(rawxm.t);
                },
            },
            None => {},
        }
    }

    fn release_header(&mut self) {
        let t0 = self.t0.unwrap();

        // obtain new file, release header
        let mut fd = BufWriter::new(self.fd(t0));

        let header = self.build_header();

        header.format(&mut fd).unwrap_or_else(|e| {
            panic!(
                "RINEX header formatting: {}. Aborting (avoiding corrupt file)",
                e
            )
        });

        let _ = fd.flush();

        self.fd = Some(fd);
        self.header = Some(header.obs.unwrap().clone());
    }

    fn release_epoch(&mut self) {
        let t = self.t.unwrap();

        let key = ObsKey {
            epoch: t,
            flag: EpochFlag::Ok, // TODO,
        };

        let mut fd = self.fd.as_mut().unwrap();

        let header = self
            .header
            .as_ref()
            .expect("internal error: missing Observation header");

        match self
            .buf
            .format(self.settings.major == 2, &key, header, &mut fd)
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
}
