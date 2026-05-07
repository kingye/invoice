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
    use super::*;
    use std::io::Read;

    #[test]
    fn test_create_archive_basic() {
        let dir = tempfile::tempdir().unwrap();

        let detail_path = dir.path().join("明细表.xlsx");
        let summary_path = dir.path().join("汇总表.xlsx");
        std::fs::write(&detail_path, b"detail content").unwrap();
        std::fs::write(&summary_path, b"summary content").unwrap();

        let att_file = dir.path().join("contract.pdf");
        std::fs::write(&att_file, b"pdf data").unwrap();

        let output_path = dir.path().join("close_2026-04.zip");

        let attachments = vec![AttachmentEntry {
            invoice_number: "FP001".to_string(),
            filepath: att_file.to_str().unwrap().to_string(),
            filename: "contract.pdf".to_string(),
        }];

        let result = create_archive(
            detail_path.to_str().unwrap(),
            summary_path.to_str().unwrap(),
            &attachments,
            output_path.to_str().unwrap(),
        );
        assert!(result.is_ok());
        assert!(output_path.exists());

        let file = std::fs::File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("明细表.xlsx").is_ok());
        assert!(archive.by_name("汇总表.xlsx").is_ok());
        assert!(archive.by_name("attachments/FP001/contract.pdf").is_ok());

        let mut detail = archive.by_name("明细表.xlsx").unwrap();
        let mut detail_content = String::new();
        detail.read_to_string(&mut detail_content).unwrap();
        assert_eq!(detail_content, "detail content");
    }

    #[test]
    fn test_create_archive_missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("output.zip");
        let result = create_archive(
            "/nonexistent/detail.xlsx",
            "/nonexistent/summary.xlsx",
            &[],
            output_path.to_str().unwrap(),
        );
        assert!(result.is_err());
    }
}
