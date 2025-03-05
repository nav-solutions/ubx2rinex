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
    rx: Receiver<Message>,
    release_header: bool,
    state: State,
}

impl Collecter {
    /// Builds new [Collecter]
    pub fn new(settings: Settings, rx: Receiver<Message>) -> Self {
        Self {
            rx,
            fd: None,
            t: None,
            t0: None,
            settings,
            release_header: true,
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
                },
            }
        }

        match self.state {
            State::Header => {
                let t0 = self.t0.expect("internal error: initial epoch");
                let mut fd = self.fd(t0);
            },
        }
    }
}

//         let constellations = self
//             .buf
//             .signals
//             .iter()
//             .map(|sig| sig.sv.constellation)
//             .unique()
//             .sorted()
//             .collect::<Vec<_>>();

//         for constellation in constellations.iter() {
//             let observables = self
//                 .buf
//                 .signals
//                 .iter()
//                 .filter_map(|sig| {
//                     if sig.sv.constellation == *constellation {
//                         Some(sig.observable.clone())
//                     } else {
//                         None
//                     }
//                 })
//                 .unique()
//                 .sorted()
//                 .collect::<Vec<_>>();

//             if let Some(ref mut obs_header) = self.rinex.header.obs {
//                 obs_header.codes.insert(*constellation, observables.clone());
//                 obs_header.timeof_first_obs = Some(self.t);
//             }
//         }

//         self.rinex.header.format(w).unwrap_or_else(|e| {
//             panic!("RINEX header formatting: {}. Aborting: corrupt RINEX", e);
//         });

//         self.release_header = false;
//     }

//     fn release(&mut self) {
//         // not optimized.. generate only once please
//         let mut filename = self.prefix.clone();
//         filename.push('/');

//         if self.major < 3 {
//             filename.push_str(&v2_filename(self.crinex, self.gzip, self.t, &self.name));
//         } else {
//             filename.push_str(&v3_filename(
//                 self.crinex,
//                 self.gzip,
//                 &self.name,
//                 "FRA",
//                 self.t,
//                 Duration::from_days(1.0),
//                 Duration::from_seconds(30.0),
//             ));
//         }

//         debug!("Filename: \"{}\"", filename);

//         let mut writer = match OpenOptions::new().write(true).read(true).open(&filename) {
//             Ok(mut fd) => {
//                 fd.seek(SeekFrom::End(0))
//                     .unwrap_or_else(|e| panic!("Failed to append to file: {}", e));

//                 BufWriter::new(fd)
//             },
//             Err(e) => {
//                 let mut fd = File::create(&filename)
//                     .unwrap_or_else(|e| panic!("Header release: failed to create file: {}", e));

//                 let mut writer = BufWriter::new(fd);
//                 self.release_header(&mut writer);
//                 writer
//             },
//         };

//         let key = ObsKey {
//             epoch: self.t,
//             flag: EpochFlag::Ok,
//         };

//         let header = self
//             .rinex
//             .header
//             .obs
//             .as_ref()
//             .expect("internal error: invalid OBS RINEX");

//         match self.buf.format(self.major == 2, &key, &header, &mut writer) {
//             Ok(_) => {
//                 // try to release internal buffer
//                 // so content is available to user rapidly
//                 let _ = writer.flush();
//             },
//             Err(e) => error!("rinex formatting error: {}", e),
//         }

//         trace!("released epoch {}", self.t);

//         // clear
//         self.buf.clock = None;
//         self.buf.signals.clear();
//     }

//     /// Call this on any new [Rawxm] measurement
//     pub fn new_observation(&mut self, t: Epoch, sv: SV, freq_id: u8, rawxm: Rawxm) {
//         trace!("{} - ({} RAWX) - {}", t, sv, rawxm);
//         if t > self.t {
//             if self.buf.signals.len() > 0 || self.buf.clock.is_some() {
//                 self.release();
//             }
//             self.t = t;
//         }

//         let c1c = if self.major == 3 {
//             Observable::from_str("C1C").unwrap()
//         } else {
//             Observable::from_str("C1").unwrap()
//         };

//         let l1c = if self.major == 3 {
//             Observable::from_str("L1C").unwrap()
//         } else {
//             Observable::from_str("L1").unwrap()
//         };

//         let d1c = if self.major == 3 {
//             Observable::from_str("D1C").unwrap()
//         } else {
//             Observable::from_str("D1").unwrap()
//         };

//         //if let Some(carrier) = freq_id_to_carrier(sv.constellation, freq_id) {
//         //    if let Some(observable) = Observable::from_carrier(sv.constellation, carrier) {
//         self.buf.signals.push(SignalObservation {
//             sv,
//             lli: None,
//             snr: None,
//             value: rawxm.cp,
//             observable: c1c,
//         });

//         self.buf.signals.push(SignalObservation {
//             sv,
//             lli: None,
//             snr: None,
//             value: rawxm.pr,
//             observable: l1c,
//         });

//         self.buf.signals.push(SignalObservation {
//             sv,
//             lli: None,
//             snr: None,
//             value: rawxm.dop as f64,
//             observable: d1c,
//         });
//     }
// }
