use serialport::SerialPort;
use std::fs::File;
use std::io::{Read, Write};

/// [Interface] to the U-Blox stream
pub enum Interface {
    /// [Interface::ReadOnly] is dedicated to read only input, mainly File inputs.
    ReadOnly(Box<dyn Read>),

    /// [Interface::Port] is used to connect to a physical port,
    /// and activately operate a U-Blox GNSS.
    Port(Box<dyn SerialPort>),
}

impl Interface {
    /// Creates a new [SerialPort] interface
    pub fn from_serial_port(port: Box<dyn SerialPort>) -> Self {
        Self::Port(port)
    }

    /// Creates a new Read-Only interface
    pub fn from_file_handle(handle: File) -> Self {
        Self::ReadOnly(Box::new(handle))
    }
}

impl std::io::Read for Interface {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::ReadOnly(r) => r.read(buf),
            Self::Port(port) => port.read(buf),
        }
    }
}

impl std::io::Write for Interface {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::ReadOnly(r) => Ok(0),
            Self::Port(port) => port.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::ReadOnly(r) => Ok(()),
            Self::Port(port) => port.flush(),
        }
    }
}
