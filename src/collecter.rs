use std::{
    fs::File,
    io::{BufWriter, Write},
    str::FromStr,
};

use itertools::Itertools;

use log::{debug, error, trace};

use rinex::{
    observation::HeaderFields as ObsHeader,
    prelude::{
        obs::{EpochFlag, ObsKey, Observations, SignalObservation},
        Epoch, Header, Observable, SV,
    },
};

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
    dump_header: bool,
    header: Header,
    obs_header: ObsHeader,
    obs_fd: BufWriter<File>,
    obs_buf: Observations,
}

impl Collecter {
    pub fn new(t0: Epoch, header: Header) -> Self {
        let obs_header = header
            .obs
            .as_ref()
            .expect("Internal error: invalid Observation definitions")
            .clone();

        Self {
            t: t0,
            header,
            obs_header,
            dump_header: true,
            obs_fd: {
                let fd = File::create("test.rnx")
                    .unwrap_or_else(|e| panic!("failed to create observation file: {}", e));
                BufWriter::new(fd)
            },
            obs_buf: Observations::default(),
        }
    }

    /// Call this on any new [Rawxm] measurement
    pub fn new_observation(&mut self, t: Epoch, sv: SV, rawxm: Rawxm) {
        if t > self.t {
            trace!("{} - NEW EPOCH - ({} RAWX) - {}", t, sv, rawxm);

            if self.obs_buf.signals.len() > 0 || self.obs_buf.clock.is_some() {
                if self.dump_header {
                    let constellations = self
                        .obs_buf
                        .signals
                        .iter()
                        .map(|sig| sig.sv.constellation)
                        .unique()
                        .sorted()
                        .collect::<Vec<_>>();

                    for constellation in constellations.iter() {
                        let observables = self
                            .obs_buf
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

                        if let Some(ref mut obs_header) = self.header.obs {
                            obs_header.codes.insert(*constellation, observables.clone());

                            obs_header.timeof_first_obs = Some(self.t);
                        }
                    }

                    self.header.format(&mut self.obs_fd).unwrap_or_else(|e| {
                        panic!(
                            "Failed to dump RINEX header: {}. Aborting, avoiding corrupt RINEX",
                            e
                        )
                    });

                    self.dump_header = false;
                }

                let key = ObsKey {
                    epoch: self.t,
                    flag: EpochFlag::Ok,
                };

                match self.obs_buf.format(
                    self.header.version.major == 2,
                    &key,
                    &self.obs_header,
                    &mut self.obs_fd,
                ) {
                    Ok(_) => {
                        debug!("dumped new observations: {:?}", self.t);
                        let _ = self.obs_fd.flush();
                    },
                    Err(e) => error!("rinex formatting error: {}", e),
                }

                self.obs_buf.clock = None;
                self.obs_buf.signals.clear();
            }

            self.t = t;
        } else {
            trace!("{} - ({} RAWX) - {}", t, sv, rawxm);
        }

        self.obs_buf.signals.push(SignalObservation {
            value: rawxm.cp,
            observable: Observable::from_str("L1C").unwrap(),
            sv,
            lli: None,
            snr: None,
        });

        self.obs_buf.signals.push(SignalObservation {
            value: rawxm.pr,
            observable: Observable::from_str("C1C").unwrap(),
            sv,
            lli: None,
            snr: None,
        });

        self.obs_buf.signals.push(SignalObservation {
            value: rawxm.dop as f64,
            observable: Observable::from_str("D1C").unwrap(),
            sv,
            lli: None,
            snr: None,
        });
    }
}
