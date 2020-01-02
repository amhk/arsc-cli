use std::convert::From;
use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    CorruptData(String),
    UnexpectedChunk,
    IoError(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            _ => todo!(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}
