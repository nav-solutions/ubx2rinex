use std::{
    collections::BTreeMap,
    io::{BufWriter, Write},
};

use log::{debug, error, info};

use rinex::{
    error::FormattingError,
    navigation::{NavFrame, NavFrameType, NavKey, NavMessageType},
    prelude::{Constellation, Epoch, Header, RinexType, Version},
    record::Record,
};

use tokio::{sync::mpsc::Receiver as Rx, sync::watch::Receiver as WatchRx};

use crate::{
    collecter::{fd::FileDescriptor, settings::Settings, Message},
    UbloxSettings,
};

pub struct Collecter {
    /// Deploy [Epoch]
    deploy_epoch: Epoch,

    /// Current [Epoch]
    epoch: Epoch,

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
}

impl Collecter {
    /// Builds new [Collecter]
    pub fn new(
        epoch: Epoch,
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
            epoch: epoch,
            deploy_epoch: epoch,
            header_released: false,
            header_comments: Default::default(),
        }
    }

    /// Obtain a new file descriptor
    fn fd(&self) -> FileDescriptor {
        let filename = self.settings.filename(true, self.epoch);
        FileDescriptor::new(self.settings.gzip, &filename)
    }

    pub async fn run(&mut self) {
        loop {
            match self.rx.recv().await {
                Some(msg) => match msg {
                    Message::EndofEpoch(t) => {},

                    Message::FirmwareVersion(version) => {},

                    Message::Ephemeris((epoch, sv, ephemeris)) => {
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

                        let key = NavKey {
                            sv,
                            epoch,
                            msgtype: NavMessageType::LNAV,
                            frmtype: NavFrameType::Ephemeris,
                        };

                        debug!("{}({}) - new LNAV ephemeris", epoch, sv);

                        let frame = NavFrame::EPH(ephemeris);

                        // let rec = self
                        //     .record
                        //     .as_mut_nav()
                        //     .expect("internal error: invalid nav setup");

                        // rec.insert(key, frame);

                        self.epoch = epoch; // update
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
}
