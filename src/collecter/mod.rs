use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Seek, SeekFrom, Write},
    os::linux::raw,
    str::FromStr,
};

use ublox::{AlignmentToReferenceTime, MonGnssConstellMask};

use rinex::{
    prelude::{Carrier, Constellation, Duration, Epoch, TimeScale},
    production::{FFU, PPU},
};

use tokio::sync::mpsc::Receiver;

use itertools::Itertools;
use log::{debug, error, trace};

use rinex::{
    observation::HeaderFields as ObsHeader,
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Header, Observable, Rinex, SV,
    },
};

pub mod rawxm;
pub mod settings;

use rawxm::Rawxm;
use settings::Settings;

#[derive(Debug, Default, Copy, Clone)]
enum State {
    #[default]
    Header,
    Observations,
}

pub enum Message {
    EndOfEpoch,
    Measurement(Rawxm),
}

pub struct Collecter {
    t: Option<Epoch>,
    t0: Option<Epoch>,
    buf: Observations,
    settings: Settings,
    fd: Option<File>,
    header: Header,
    rx: Receiver<Message>,
    state: State,
}

impl Collecter {
    /// Builds new [Collecter]
    pub fn new(settings: Settings, rx: Receiver<Message>) -> Self {
        let header = settings.header();
        Self {
            rx,
            fd: None,
            t: None,
            t0: None,
            header,
            settings,
            buf: Observations::default(),
            state: Default::default(),
        }
    }

    /// Obtain a new file descriptor
    fn fd(&self, t: Epoch) -> BufWriter<File> {
        let filename = self.settings.filename(t);

        let fd = File::create(&filename)
            .unwrap_or_else(|e| panic!("Failed to open \"{}\": {}", filename, e));

        if self.settings.gzip {
            BufWriter::new(fd)
        } else {
            BufWriter::new(fd)
        }
    }

    pub async fn run(&mut self) {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                Message::EndOfEpoch => {},
                Message::Measurement(rawxm) => {
                    if self.t0.is_none() {
                        self.t0 = Some(rawxm.t);
                    }

                    self.t = Some(rawxm.t);

                    let c1c = if self.major == 3 {
                        Observable::from_str("C1C").unwrap()
                    } else {
                        Observable::from_str("C1").unwrap()
                    };

                    let l1c = if self.major == 3 {
                        Observable::from_str("L1C").unwrap()
                    } else {
                        Observable::from_str("L1").unwrap()
                    };

                    let d1c = if self.major == 3 {
                        Observable::from_str("D1C").unwrap()
                    } else {
                        Observable::from_str("D1").unwrap()
                    };

                    self.buf.signals.push(SignalObservation {
                        sv,
                        lli: None,
                        snr: None,
                        value: rawxm.cp,
                        observable: c1c,
                    });

                    self.buf.signals.push(SignalObservation {
                        sv,
                        lli: None,
                        snr: None,
                        value: rawxm.pr,
                        observable: l1c,
                    });

                    self.buf.signals.push(SignalObservation {
                        sv,
                        lli: None,
                        snr: None,
                        value: rawxm.dop as f64,
                        observable: d1c,
                    });
                },
            }
        }

        match self.state {
            State::Header => {
                if self.t0.is_none() {
                    continue; // too early
                }

                let t0 = self.t0.unwrap();

                // obtain new file, release header
                let mut fd = self.fd(t0);

                let header = self.header();

                header.format(&mut fd).unwrap_or_else(|e| {
                    panic!(
                        "RINEX header formatting: {}. Aborting (avoiding corrupt file)",
                        e
                    )
                });

                self.state = State::Observations;
            },
            State::Observations => {
                let key = ObsKey {
                    epoch: self.t,
                    flag: EpochFlag::Ok, // TODO,
                };

                let mut fd = self.fd.unwrap();

                match self
                    .buf
                    .format(self.settings.major == 2, &key, &self.header, &mut fd)
                {
                    Ok(_) => {
                        let _ = fd.flush();
                        self.buf.clock = None;
                        self.buf.signals.clear();
                    },
                    Err(e) => {
                        error!("{} formatting issue: {}", self.t, e);
                    },
                }
            },
        }
    }

    fn header(&self) -> Header {
        let mut header = Header::default();

        // TODO: observables need to be based on Ublox caps
        if let Some(operator) = &self.settings.operator {
            header.observer = Some(operator.clone());
        }

        header
    }
}
