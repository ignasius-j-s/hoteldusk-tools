use hoteldusk_tools::util::ReadExt;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let txt_files = std::env::args_os()
        .skip(1)
        .filter(|arg| {
            Path::new(&arg)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
        })
        .collect::<Vec<_>>();

    if txt_files.is_empty() {
        println!("Usage: txt_decoder txt_file(s) ...");
        return Ok(());
    }

    for txt_file in &txt_files {
        let mut file = File::open(txt_file)?;
        let lines_count: u32 = file.read_le()?;
        let lines_start = 4 + lines_count * 4;

        let mut buf = vec![0; 4 * lines_count as usize];
        file.read_exact(&mut buf)?;
        let mut lines_table = Cursor::new(buf);

        let mut buf_reader = BufReader::new(&file);
        let mut buf_writer = Cursor::new(Vec::new());
        for _ in 0..lines_count {
            let mut lines_buf = Vec::new();
            let offset = lines_start + lines_table.read_le::<u32>()?;

            buf_reader.seek(SeekFrom::Start(offset as u64))?;
            buf_reader.read_until(0, &mut lines_buf)?;

            // remove null byte
            lines_buf.pop();
            buf_writer.write_all(&lines_buf)?;
            buf_writer.write_all(b"\n")?;
        }

        std::fs::write(txt_file, buf_writer.into_inner())?;
    }

    Ok(())
}
