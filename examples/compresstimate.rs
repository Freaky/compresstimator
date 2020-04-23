use std::io::Write;
use std::time::Instant;

use compresstimator::Compresstimator;

fn main() -> std::io::Result<()> {
    let estimator = Compresstimator::default();

    for path in std::env::args_os().skip(1) {
        let path = std::path::PathBuf::from(path);

        print!("{}\t", path.display());

        let start = Instant::now();
        match estimator.compresstimate_file(&path) {
            Ok(ratio) => {
                print!("Est ({:.2?}): {:.2}x\t", start.elapsed(), ratio);
            }
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        }

        std::io::stdout().flush()?;

        let start = Instant::now();
        match std::fs::File::open(&path).and_then(|file| estimator.base_truth(file)) {
            Ok(ratio) => {
                println!("Actual ({:.2?}): {:.2}x", start.elapsed(), ratio);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}
