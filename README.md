# Compresstimator

[![Crates.io](https://img.shields.io/crates/v/compresstimator.svg)](https://crates.io/crates/compresstimator)

A simple vaguely statistical file compressibility tester.

## Description

`Compresstimator` is a library for quickly determining the rough compressibility
of a given file, using linear sampling (i.e. testing every n-th block) and lz4
compression.

For large files this can give a reasonable idea of compressibility with just a
few dozen seeks and a few hundred kilobytes of data - usually serviced by even
slow hard disks in under a second.

## Usage

`Compresstimator` consists of a public type that encapsulates the settings of
the estimator - currently just the block size used for sampling.  In future this
may also include settings for trading accuracy for speed.

All estimation functions return an `f32` indicating the compression ratio,
between 0 and 1.

```rust
use compresstimator::Compresstimator;

// Create an estimator with the default block size of 4096 bytes
let estimator = Compresstimator::new();

// This is usually accurate to within 10-15% for lz4 level 1
if estimator.compresstimate_file("huge_file")? > 0.95 {
  	println!("Probably doesn't compress well.");
}

// If you have the file size, you can avoid a metadata lookup
let len = fs::metadata("huge_file")?.len();
if estimator.compresstimate_file_len("huge_file", len)? > 0.95 {
  	println!("Probably doesn't compress well.");
}

// You can also pass in a handle to any Read + Seek
let mut f = File::open("huge_file")?;
if estimator.compresstimate(&mut f, len)? > 0.95 {
  	println!("Probably doesn't compress well.");
}
```
