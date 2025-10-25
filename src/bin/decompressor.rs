use hoteldusk_tools::util::decompress;
use std::{error::Error, fs::File, io::BufReader};

fn main() -> Result<(), Box<dyn Error>> {
    let files = std::env::args_os()
        .skip(1)
        .filter(|arg| std::fs::metadata(arg).is_ok_and(|md| md.is_file()))
        .collect::<Vec<_>>();

    if files.is_empty() {
        println!("Usage: decompressor file(s) ...");
        return Ok(());
    }

    for path in &files {
        let file = File::open(path)?;
        let reader = &mut BufReader::new(file);

        match decompress(reader) {
            Ok(data) => std::fs::write(path, data)?,
            Err(_) => continue,
        }
    }

    Ok(())
}
