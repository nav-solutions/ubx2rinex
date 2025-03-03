use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Seek, SeekFrom, Write},
    str::FromStr,
};

use itertools::Itertools;

use log::{debug, error, trace};

use rinex::{
    observation::HeaderFields as ObsHeader,
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Duration, Epoch, Header, Observable, Rinex, SV,
    },
};

use crate::utils::{v2_filename, v3_filename};

pub struct Rawxm {
    pub pr: f64,
    pub cp: f64,
    pub dop: f32,
    pub cno: u8,
}

impl std::fmt::Display for Rawxm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pr={:.7E} cp={:.7E} dop={:.7E} cno={}",
            self.pr, self.cp, self.dop, self.cno,
        )
    }
}

impl Rawxm {
    pub fn new(pr: f64, cp: f64, dop: f32, cno: u8) -> Self {
        Self { pr, cp, dop, cno }
    }
}

pub struct Collecter {
    t: Epoch,
    prefix: String,
    major: u8,
    gzip: bool,
    crinex: bool,
    rinex: Rinex,
    name: String,
    buf: Observations,
    release_header: bool,
}

impl Collecter {
    /// Builds new [Collecter]
    pub fn new(prefix: &str, t0: Epoch, rinex: Rinex, crinex: bool, gzip: bool) -> Self {
        let major = rinex.header.version.major;

        Self {
            t: t0,
            major,
            rinex,
            crinex,
            gzip,
            prefix: prefix.to_string(),
            name: String::from("UBX"),
            release_header: true,
            buf: Observations::default(),
        }
    }

    /// Release (publish) file header
    fn release_header<W: Write>(&mut self, w: &mut BufWriter<W>) {
        // trick to render a meaningful file name (using RINEX V0.17)
        let rec = self
            .rinex
            .record
            .as_mut_obs()
            .expect("internal error while release Header");

        rec.insert(
            ObsKey {
                epoch: self.t,
                flag: EpochFlag::Ok,
            },
            Default::default(),
        );

        // last header customizations
        let constellations = self
            .buf
            .signals
            .iter()
            .map(|sig| sig.sv.constellation)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        for constellation in constellations.iter() {
            let observables = self
                .buf
                .signals
                .iter()
                .filter_map(|sig| {
                    if sig.sv.constellation == *constellation {
                        Some(sig.observable.clone())
                    } else {
                        None
                    }
                })
                .unique()
                .sorted()
                .collect::<Vec<_>>();

            if let Some(ref mut obs_header) = self.rinex.header.obs {
                obs_header.codes.insert(*constellation, observables.clone());
                obs_header.timeof_first_obs = Some(self.t);
            }
        }

        self.rinex.header.format(w).unwrap_or_else(|e| {
            panic!("RINEX header formatting: {}. Aborting: corrupt RINEX", e);
        });

        self.release_header = false;
    }

    fn release(&mut self) {
        // not optimized.. generate only once please
        let mut filename = self.prefix.clone();
        filename.push('/');

        if self.major < 3 {
            filename.push_str(&v2_filename(self.crinex, self.gzip, self.t, &self.name));
        } else {
            filename.push_str(&v3_filename(
                self.crinex,
                self.gzip,
                &self.name,
                "FRA",
                self.t,
                Duration::from_days(1.0),
                Duration::from_seconds(30.0),
            ));
        }

        debug!("Filename: \"{}\"", filename);

        let mut writer = match OpenOptions::new().write(true).read(true).open(&filename) {
            Ok(mut fd) => {
                fd.seek(SeekFrom::End(0))
                    .unwrap_or_else(|e| panic!("Failed to append to file: {}", e));

                BufWriter::new(fd)
            },
            Err(e) => {
                let mut fd = File::create(&filename)
                    .unwrap_or_else(|e| panic!("Header release: failed to create file: {}", e));

                let mut writer = BufWriter::new(fd);
                self.release_header(&mut writer);
                writer
            },
        };

        let key = ObsKey {
            epoch: self.t,
            flag: EpochFlag::Ok,
        };

        let header = self
            .rinex
            .header
            .obs
            .as_ref()
            .expect("internal error: invalid OBS RINEX");

        match self.buf.format(self.major == 2, &key, &header, &mut writer) {
            Ok(_) => {
                // try to release internal buffer
                // so content is available to user rapidly
                let _ = writer.flush();
            },
            Err(e) => error!("rinex formatting error: {}", e),
        }

        trace!("released epoch {}", self.t);

        // clear
        self.buf.clock = None;
        self.buf.signals.clear();
    }

    /// Call this on any new [Rawxm] measurement
    pub fn new_observation(&mut self, t: Epoch, sv: SV, freq_id: u8, rawxm: Rawxm) {
        trace!("{} - ({} RAWX) - {}", t, sv, rawxm);
        if t > self.t {
            if self.buf.signals.len() > 0 || self.buf.clock.is_some() {
                self.release();
            }
            self.t = t;
        }

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

        //if let Some(carrier) = freq_id_to_carrier(sv.constellation, freq_id) {
        //    if let Some(observable) = Observable::from_carrier(sv.constellation, carrier) {
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
    }
}
