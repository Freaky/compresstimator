use std::time::Instant;

use compresstimator::Compresstimator;

fn main() -> std::io::Result<()> {
    let estimator = Compresstimator::default();

    for path in std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_file())
    {
        println!("Path: {}", path.display());

        let start = Instant::now();
        let est = estimator.compresstimate_file(&path)?;
        let est_time = start.elapsed();

        println!("  Estimate: {:.2}x, Time: {:.2?}", est, est_time);

        let start = Instant::now();
        let act = std::fs::File::open(&path).and_then(|file| estimator.base_truth(file))?;
        let act_time = start.elapsed();

        println!("    Actual: {:.2}x, Time: {:.2?}", act, act_time);
    }

    Ok(())
}
