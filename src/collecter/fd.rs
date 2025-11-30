use flate2::{Compression, write::GzEncoder};
use std::fs::File;

pub enum FileDescriptor {
    Plain(File),
    Gzip(GzEncoder<File>),
}

impl std::io::Write for FileDescriptor {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(w) => w.write(data),
            Self::Gzip(w) => w.write(data),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(w) => w.flush(),
            Self::Gzip(w) => w.flush(),
        }
    }
}

impl FileDescriptor {
    pub fn new(gzip: bool, filename: &str) -> Self {
        let fd = File::create(&filename)
            .unwrap_or_else(|e| panic!("Failed to open \"{}\": {}", filename, e));

        if gzip {
            let compression = Compression::new(5);
            Self::Gzip(GzEncoder::new(fd, compression))
        } else {
            Self::Plain(fd)
        }
    }
}
