
use compresstimator::Compresstimator;

fn main() -> std::io::Result<()> {
    let estimator = Compresstimator::default();

    for path in std::env::args_os().skip(1) {
        let path = std::path::PathBuf::from(path);

        print!("{}\t", path.display());

        match estimator.compresstimate_file(&path) {
            Ok(ratio) => {
                println!("{:.2}x", ratio);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}
