use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

use crate::db;
use crate::models;

pub fn add_attachment(
    conn: &rusqlite::Connection,
    invoice_id: i64,
    invoice_number: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let data_dir = cwd.join(format!(".invoice/data/{}", invoice_number));
    std::fs::create_dir_all(&data_dir)?;

    let src_path = Path::new(file_path);
    let filename = src_path
        .file_name()
        .ok_or("Cannot determine filename")?
        .to_string_lossy()
        .to_string();

    let dest_path = data_dir.join(&filename);

    let mut src_file = std::fs::File::open(src_path)?;
    let mut dest_file = std::fs::File::create(&dest_path)?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 4096];
    let mut total_written: u64 = 0;
    loop {
        let bytes_read = src_file.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        dest_file.write_all(&buf[..bytes_read])?;
        hasher.update(&buf[..bytes_read]);
        total_written += bytes_read as u64;
    }

    let hash_hex = format!("{:x}", hasher.finalize());
    let db_path = format!(".invoice/data/{}/{}", invoice_number, filename);

    let att = models::Attachment {
        invoice_id,
        filename,
        filepath: db_path,
        file_hash: hash_hex,
        file_size: total_written as i64,
        ..Default::default()
    };
    db::insert_attachment(conn, &att)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_add_attachment_placeholder() {
        assert!(true);
    }
}
