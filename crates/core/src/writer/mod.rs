#[cfg(feature = "file")]
pub mod file;

use crate::ProgressEntry;
use bytes::Bytes;
use std::future::Future;

pub trait SeqWriter: Send {
    fn write_sequentially(
        &mut self,
        bytes: &Bytes,
    ) -> impl Future<Output = Result<(), std::io::Error>> + Send;
    fn flush(&mut self) -> impl Future<Output = Result<(), std::io::Error>> + Send;
}

pub trait RandWriter: Send {
    fn write_randomly(
        &mut self,
        range: ProgressEntry,
        bytes: &Bytes,
    ) -> impl Future<Output = Result<(), std::io::Error>> + Send;
    fn flush(&mut self) -> impl Future<Output = Result<(), std::io::Error>> + Send;
}
