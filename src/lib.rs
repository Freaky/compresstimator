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

#[derive(Debug, Clone, Copy)]
enum Confidence {
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

fn sample_size(pop: u64, moe: u8, confidence: Confidence) -> f32 {
    let pop = pop as f32;
    let n_naught = 0.25 * (f32::from(confidence) / (f32::from(moe) / 100.0)).powi(2);
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
    error_margin: u8,
    confidence: Confidence,
}

const DEFAULT_BLOCK_SIZE: u64 = 4096;

impl Default for Compresstimator {
    /// Create a `Compresstimator` with a default block size of 4096 bytes
    fn default() -> Self {
        Self {
            block_size: DEFAULT_BLOCK_SIZE,
            error_margin: 15,
            confidence: Confidence::C90,
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
    pub fn with_block_size(block_size: usize) -> Self {
        Self {
            block_size: block_size as u64,
            ..Self::default()
        }
    }

    /// Exhaustively compress the file and return the achieved ratio.  This is
    /// really only for validation purposes during testing.
    pub fn base_truth<P: AsRef<Path>>(&self, path: P) -> io::Result<f32> {
        let mut input = File::open(path)?;

        let output = WriteCount::default();
        let mut encoder = EncoderBuilder::new().level(1).build(output)?;
        let written = std::io::copy(&mut input, &mut encoder)?;

        let (output, result) = encoder.finish();
        result.map(|_| output.written as f32 / written as f32)
    }

    /// Compresstimate the seekable stream `input` of `len` bytes, returning an
    /// estimated conservative compress ratio (based on lz4 level 1).
    pub fn compresstimate<P: Read + Seek>(&self, mut input: P, len: u64) -> io::Result<f32> {
        let output = WriteCount::default();

        let mut encoder = EncoderBuilder::new().level(1).build(output)?;

        let blocks = len / self.block_size;
        let samples = sample_size(blocks, 15, Confidence::C90) as u64;
        let written;

        // If we're going to be randomly sampling a big chunk of the file anyway,
        // we might as well read in the lot.
        if samples == 0 || len < samples * self.block_size * 4 {
            written = std::io::copy(&mut input, &mut encoder)?;
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
        result.map(|_| output.written as f32 / written as f32)
    }

    /// Compresstimate a path with a known file length.
    pub fn compresstimate_file_len<P: AsRef<Path>>(&self, path: P, len: u64) -> io::Result<f32> {
        self.compresstimate(File::open(path)?, len)
    }

    /// Compresstimate a path.
    pub fn compresstimate_file<P: AsRef<Path>>(&self, path: P) -> io::Result<f32> {
        let input = File::open(path)?;
        let len = input.metadata()?.len();
        self.compresstimate(input, len)
    }
}

#[test]
fn amazing_test_suite() {
    let est = Compresstimator::default();

    assert!(est.compresstimate_file("Cargo.lock").expect("Cargo.lock") < 1.0);

    if std::path::PathBuf::from("/dev/urandom").exists() {
        assert!(est.compresstimate_file_len("/dev/urandom", 1024 * 1024).expect("/dev/urandom") >= 1.0);
    }
}
