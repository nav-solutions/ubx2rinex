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

use tokio::{
    signal,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        watch,
    },
    time::sleep,
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
    rx: Receiver<Message>,
}

impl Collecter {

}