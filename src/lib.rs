#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]

//! # Compresstimator
//!
//! Simple file compressibility estimation
//!

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use lz4::EncoderBuilder;

#[derive(Debug, Default)]
struct WriteCount {
    written: u64,
}

impl Write for WriteCount {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written += buf.len() as u64;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A statistical confidence level, 80% - 99%
#[derive(Debug, Clone, Copy)]
pub enum Confidence {
    C80,
    C85,
    C90,
    C95,
    C99,
}

impl From<Confidence> for f32 {
    fn from(c: Confidence) -> f32 {
        match c {
            Confidence::C80 => 1.28,
            Confidence::C85 => 1.44,
            Confidence::C90 => 1.65,
            Confidence::C95 => 1.96,
            Confidence::C99 => 2.58,
        }
    }
}

fn sample_size(pop: u64, moe: f32, confidence: Confidence) -> f32 {
    let pop = pop as f32;
    let n_naught = 0.25 * (f32::from(confidence) / moe).powi(2);
    ((pop * n_naught) / (n_naught + pop - 1.0)).ceil()
}

/// A compression estimator with a configured block size, and (currently) fixed
/// accuracy (±15%, 90% confidence)
///
/// ```no_run
/// use compresstimator::Compresstimator;
///
/// let est = Compresstimator::default();
/// match est.compresstimate_file("big_file.dat") {
///     Ok(ratio) => println!("Compression ratio: {}", ratio),
///     Err(e) => eprintln!("IO Error: {}", e)
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Compresstimator {
    block_size: u64,
    error_margin: f32,
    confidence: Confidence,
}

const DEFAULT_BLOCK_SIZE: u64 = 4096;

impl Default for Compresstimator {
    /// Create a `Compresstimator` with a default block size of 4096 bytes,
    /// 10% margin of error, and 95% confidence level.
    fn default() -> Self {
        Self {
            block_size: DEFAULT_BLOCK_SIZE,
            error_margin: 0.1,
            confidence: Confidence::C95,
        }
    }
}

impl Compresstimator {
    /// Alias for `default()`
    pub fn new() -> Self {
        Self::default()
    }

    /// Use a given block size for compresstimation.  This should be some reasonable
    /// multiple of the underlying filesystem block size.
    pub fn with_block_size(block_size: u64) -> Self {
        Self {
            block_size: block_size as u64,
            ..Self::default()
        }
    }

    /// Use a given block size for compresstimation.  This should be some reasonable
    /// multiple of the underlying filesystem block size.
    pub fn block_size(&mut self, block_size: u64) -> &Self {
        self.block_size = block_size;
        self
    }

    /// Set the margin of error for the compressibility check.
    ///
    /// # Panics
    ///
    /// Panics if the error margin is not between 0 and 1.
    pub fn error_margin(&mut self, margin: f32) -> &Self {
        assert!(margin > 0.0 && margin < 1.0);
        self.error_margin = margin;
        self
    }

    /// Set the confidence level of the compressibility check.
    pub fn confidence_level(&mut self, confidence: Confidence) -> &Self {
        self.confidence = confidence;
        self
    }

    /// Exhaustively compress the stream and return the achieved ratio.
    pub fn base_truth<R: Read>(&self, mut input: R) -> io::Result<f32> {
        let output = WriteCount::default();
        let mut encoder = EncoderBuilder::new().level(1).build(output)?;
        let written = std::io::copy(&mut input, &mut encoder)?;

        let (output, result) = encoder.finish();
        result.map(|_| (output.written as f32 / written as f32).min(1.0))
    }

    /// Compresstimate the seekable stream `input` from the current position to the
    /// end.
    ///
    /// This function determines the length of the stream by seeking to the end.
    pub fn compresstimate<P: Read + Seek>(&self, mut input: P) -> io::Result<f32> {
        // In future consider stream_len() and stream_position()
        // https://github.com/rust-lang/rust/issues/59359
        let pos = input.seek(SeekFrom::Current(0))?;
        let len = input.seek(SeekFrom::End(0))?;
        input.seek(SeekFrom::Start(pos))?;
        self.compresstimate_len(&mut input, len - pos)
    }

    /// Compresstimate up to `len` bytes from the seekable `input` stream,
    /// returning an estimated compression ratio (currently based on lz4 level 1).
    pub fn compresstimate_len<P: Read + Seek>(&self, mut input: P, len: u64) -> io::Result<f32> {
        let output = WriteCount::default();
        let mut encoder = EncoderBuilder::new().level(1).build(output)?;

        let blocks = len / self.block_size;
        let samples = sample_size(blocks, self.error_margin, self.confidence) as u64;
        let written;

        // If we're going to be randomly sampling a big chunk of the file anyway,
        // we might as well read in the lot.
        if samples == 0 || len < samples * self.block_size * 4 {
            written = std::io::copy(&mut input.take(len), &mut encoder)?;
        } else {
            let step = self.block_size * (blocks / samples);

            let mut buf = vec![0; self.block_size as usize];
            written = self.block_size * samples;

            for i in 0..samples {
                input.seek(SeekFrom::Start(step * i))?;
                input.read_exact(&mut buf)?;
                encoder.write_all(&buf)?;
            }
        }

        let (output, result) = encoder.finish();
        result.map(|_| (output.written as f32 / written as f32).min(1.0))
    }

    /// Compresstimate the first `len` bytes of the file located at `path`.
    ///
    /// If the file is shorter than `len`, this function may fail with a seek error.
    pub fn compresstimate_file_len<P: AsRef<Path>>(&self, path: P, len: u64) -> io::Result<f32> {
        self.compresstimate_len(File::open(path)?, len)
    }

    /// Compresstimate the file located at `path`.
    pub fn compresstimate_file<P: AsRef<Path>>(&self, path: P) -> io::Result<f32> {
        self.compresstimate(File::open(path)?)
    }
}

#[test]
fn amazing_test_suite() {
    let est = Compresstimator::default();

    assert!(est.compresstimate_file("Cargo.lock").expect("Cargo.lock") < 1.0);

    let empty = vec![];
    assert!(
        est.compresstimate(std::io::Cursor::new(empty))
            .expect("empty should work")
            == 1.0
    );

    if std::path::PathBuf::from("/dev/urandom").exists() {
        assert!(
            est.compresstimate_file_len("/dev/urandom", 1024 * 1024)
                .expect("/dev/urandom")
                >= 1.0
        );
    }
}
