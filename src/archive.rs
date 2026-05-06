use std::io::{Read, Write};
use std::path::Path;

pub struct AttachmentEntry {
    pub invoice_number: String,
    pub filepath: String,
    pub filename: String,
}

pub fn create_archive(
    detail_path: &str,
    summary_path: &str,
    attachment_paths: &[AttachmentEntry],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let file = std::fs::File::create(output_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    add_file_to_zip(&mut zip, detail_path, "明细表.xlsx", options)?;
    add_file_to_zip(&mut zip, summary_path, "汇总表.xlsx", options)?;

    for att in attachment_paths {
        let entry_path = format!("attachments/{}/{}", att.invoice_number, att.filename);
        add_file_to_zip(&mut zip, &att.filepath, &entry_path, options)?;
    }

    zip.finish()?;
    Ok(())
}

fn add_file_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    src_path: &str,
    entry_name: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(src_path);
    if !path.exists() {
        return Err(format!("Source file not found: {}", src_path).into());
    }
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    zip.start_file(entry_name, options)?;
    zip.write_all(&buf)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_create_archive_placeholder() {
        assert!(true);
    }
}
