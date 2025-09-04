use serialport::SerialPort;
use std::{fs::File, io::Read};

/// [ReadOnlyPool] is used to stack many input file descriptors
pub struct ReadOnlyPool {
    /// Current pointer
    ptr: usize,

    /// Total number of items
    size: usize,

    /// Stack
    readers: Vec<Box<dyn Read>>,
}

impl std::io::Read for ReadOnlyPool {
    // Consumes descriptors one by one, without a sense of priority.
    // Upgrade this (complexify) in case we need to manage chronology.

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.ptr == self.size {
            return Ok(0);
        }

        match self.readers[self.ptr].read(buf) {
            Ok(0) => {
                // move on to next pointer
                self.ptr += 1;
                self.read(buf)
            },
            Ok(size) => Ok(size), // pass through
            Err(e) => Err(e),     // pass through
        }
    }
}

impl ReadOnlyPool {
    pub fn new(handle: Box<dyn Read>) -> Self {
        Self {
            ptr: 0,
            size: 1,
            readers: vec![handle],
        }
    }

    pub fn stack_handle(&mut self, handle: Box<dyn Read>) {
        self.readers.push(handle);
        self.size += 1;
    }
}

/// [Interface] to the U-Blox stream
pub enum Interface {
    /// [Interface::ReadOnlyPool] is dedicated to read only file descriptors,
    /// to deserialize a U-Blox snapshot.
    ReadOnlyPool(ReadOnlyPool),

    /// [Interface::Port] is used to connect to a physical port,
    /// and activately operate a U-Blox GNSS.
    Port(Box<dyn SerialPort>),
}

impl Interface {
    /// True if this [Interface] is read-only
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::ReadOnlyPool(_))
    }

    /// Creates a new [SerialPort] interface
    pub fn from_serial_port(port: Box<dyn SerialPort>) -> Self {
        Self::Port(port)
    }

    /// Creates a new Read-Only interface
    pub fn from_file_handle(handle: File) -> Self {
        Self::ReadOnlyPool(ReadOnlyPool::new(Box::new(handle)))
    }

    /// Adds a file handle to a Read Only interface.
    /// Only applies to [Self::Port] use case.
    pub fn stack_file_handle(&mut self, handle: File) {
        match self {
            Self::Port(_) => {}, // invalid use of the API
            Self::ReadOnlyPool(pool) => pool.stack_handle(Box::new(handle)),
        }
    }
}

impl std::io::Read for Interface {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::ReadOnlyPool(r) => r.read(buf),
            Self::Port(port) => port.read(buf),
        }
    }
}

impl std::io::Write for Interface {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::ReadOnlyPool(_) => Ok(buf.len()),
            Self::Port(port) => port.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::ReadOnlyPool(_) => Ok(()),
            Self::Port(port) => port.flush(),
        }
    }
}
