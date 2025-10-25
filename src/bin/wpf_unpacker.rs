use hoteldusk_tools::util::ReadExt;
use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let wpf_files = std::env::args_os()
        .skip(1)
        .filter(|arg| {
            Path::new(&arg)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("wpf"))
        })
        .collect::<Vec<_>>();

    if wpf_files.is_empty() {
        println!("Usage: wpf_unpacker wpf_file(s) ...");
        return Ok(());
    }

    for wpf in &wpf_files {
        let mut file = BufReader::new(File::open(wpf)?);
        let output_path = Path::new(wpf).with_extension("");
        std::fs::create_dir(&output_path).ok();

        let mut name_buf = [0; 24];
        while file.read_exact(&mut name_buf).is_ok() {
            let end = name_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_buf.len());

            if end == 0 {
                break;
            }

            let name = str::from_utf8(&name_buf[1..end])?;
            let size: u32 = file.read_le()?;
            let next: u32 = file.read_le()?;

            let path = output_path.join(name);
            let mut data = vec![0; size as usize];
            file.read_exact(&mut data)?;
            file.seek(SeekFrom::Start(next as u64))?;
            std::fs::write(path, data)?;
        }
    }

    Ok(())
}
