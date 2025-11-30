use std::{
    collections::HashMap,
    io::{BufWriter, Write},
};

use log::{debug, error};

use rinex::{
    error::FormattingError,
    navigation::{Ephemeris, NavMessageType},
    prelude::{Constellation, Epoch, Header, RinexType, SV, Version},
};

use tokio::{sync::mpsc::Receiver as Rx, sync::watch::Receiver as WatchRx};

use crate::{
    UbloxSettings,
    collecter::{Message, fd::FileDescriptor, settings::Settings},
};

pub struct Collecter {
    /// First [Epoch] received from U-Blox
    first_epoch: Option<Epoch>,

    /// Latest [Epoch] received from U-Blox
    epoch: Option<Epoch>,

    /// True when Header has been released for this period
    header_released: bool,

    /// Receiver channel
    rx: Rx<Message>,

    /// Shutdown channel
    shutdown: WatchRx<bool>,

    /// Collection [Settings]
    settings: Settings,

    /// [UbloxSettings]
    ubx_settings: UbloxSettings,

    /// Custom header comments
    header_comments: Vec<String>,

    /// Current [FileDescriptor] handle
    fd: Option<BufWriter<FileDescriptor>>,

    /// Last message released, per SV
    latest_release: HashMap<SV, Epoch>,
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
            settings,
            fd: None,
            shutdown,
            ubx_settings: ublox,
            header_released: false,
            epoch: Default::default(),
            first_epoch: Default::default(),
            latest_release: Default::default(),
            header_comments: Default::default(),
        }
    }

    /// Obtain a new [FileDescriptor]
    fn fd(&self) -> FileDescriptor {
        let epoch = self.epoch.unwrap();
        let filename = self.settings.filename(true, epoch);
        FileDescriptor::new(self.settings.gzip, &filename)
    }

    pub async fn run(&mut self) {
        loop {
            match self.rx.recv().await {
                Some(msg) => match msg {
                    Message::FirmwareVersion(version) => {
                        self.ubx_settings.firmware = Some(version.to_string());
                    },

                    Message::HeaderComment(comment) => {
                        if self.header_comments.len() < 16 {
                            self.header_comments.push(comment);
                        }
                    },

                    Message::Ephemeris((epoch, sv, ephemeris)) => {
                        if self.first_epoch.is_none() {
                            self.first_epoch = Some(epoch);
                            self.epoch = Some(epoch);
                        }

                        if !self.header_released {
                            match self.release_header() {
                                Ok(_) => {
                                    debug!("{} - NAV header released", epoch);
                                },
                                Err(e) => {
                                    error!("{} - failed to redact RINEX header: {}", epoch, e);
                                    return;
                                },
                            }

                            self.header_released = true;
                        }

                        let mut do_release = false;

                        if let Some(latest) = self.latest_release.get_mut(&sv) {
                            if (epoch - *latest) >= self.settings.nav_period {
                                do_release = true;
                            }
                        } else {
                            do_release = true;
                        }

                        if do_release {
                            match self.release_message(epoch, sv, ephemeris) {
                                Ok(_) => {
                                    self.latest_release.insert(sv, epoch); // update
                                    debug!("{}({}) - published ephemeris message", epoch, sv);
                                },
                                Err(e) => {
                                    error!("{} - failed to release epoch: {}", epoch, e);
                                },
                            }
                        }

                        self.epoch = Some(epoch); // update
                    },

                    Message::Shutdown => {
                        return;
                    },

                    _ => {},
                },
                None => {},
            }
        }
    }

    fn build_header(&self) -> Header {
        let mut header = Header::default();

        // revision
        header.rinex_type = RinexType::NavigationData;
        header.version.major = self.settings.major;

        // GNSS
        if self.ubx_settings.constellations.len() == 1 {
            header.constellation = Some(self.ubx_settings.constellations[0]);
        } else {
            header.constellation = Some(Constellation::Mixed);
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

        header
    }

    fn release_header(&mut self) -> Result<(), FormattingError> {
        // obtain a file descriptor
        let mut fd = BufWriter::new(self.fd());

        let header = self.build_header();

        header.format(&mut fd)?; // must pass

        let _ = fd.flush(); // can fail
        self.fd = Some(fd);

        Ok(())
    }

    fn release_message(
        &mut self,
        epoch: Epoch,
        sv: SV,
        ephemeris: Ephemeris,
    ) -> Result<(), FormattingError> {
        let fd = self.fd.as_mut().unwrap();

        // write epoch
        let (y, m, d, hh, mm, ss, nanos) = epoch.to_gregorian(epoch.time_scale);

        let decis = nanos / 100_000;

        match self.settings.major {
            4 => {
                write!(
                    fd,
                    "> EPH {:x} {}\n{:x} {:04} {:02} {:02} {:02} {:02} {:02}",
                    sv,
                    NavMessageType::LNAV,
                    sv,
                    y,
                    m,
                    d,
                    hh,
                    mm,
                    ss
                )?;
            },
            3 => {
                write!(
                    fd,
                    "{:x} {:04} {:02} {:02} {:02} {:02} {:02}",
                    sv, y, m, d, hh, mm, ss
                )?;
            },
            _ => {
                if self.ubx_settings.constellations.len() == 1 {
                    write!(
                        fd,
                        "{:02} {:02} {:02} {:02} {:02} {:02} {:2}.{:01}",
                        sv.prn,
                        y - 2000,
                        m,
                        d,
                        hh,
                        mm,
                        ss,
                        decis
                    )?;
                } else {
                    write!(
                        fd,
                        "{:x} {:04} {:02} {:02} {:02} {:02} {:02}",
                        sv, y, m, d, hh, mm, ss
                    )?;
                }
            },
        }

        // format payload
        let version = Version::from_major(self.settings.major);
        ephemeris.format(fd, sv, version, NavMessageType::LNAV)?;

        let _ = fd.flush();

        Ok(())
    }
}
