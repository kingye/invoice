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

    let hash_hex = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();
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
    use super::*;
    use crate::db;
    use std::sync::Mutex;

    // Tests that call set_current_dir must be serialized to avoid race conditions
    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_add_attachment_and_hash() {
        let _lock = CWD_MUTEX.lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("invoice.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        db::init_schema(&conn).unwrap();

        let inv = models::Invoice {
            number: "ATT001".to_string(),
            date: "2026-05-01".to_string(),
            ..Default::default()
        };
        let id = db::insert_invoice(&conn, &inv).unwrap();

        let src_file = dir.path().join("test_file.txt");
        std::fs::write(&src_file, b"hello world attachment").unwrap();

        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = add_attachment(&conn, id, "ATT001", src_file.to_str().unwrap());
        std::env::set_current_dir(&orig_dir).unwrap();
        assert!(result.is_ok());

        let atts = db::get_attachments_for_invoice(&conn, id).unwrap();
        assert_eq!(atts.len(), 1);
        assert_eq!(atts[0].filename, "test_file.txt");
        assert_eq!(atts[0].file_size, 22);
        assert!(!atts[0].file_hash.is_empty());

        let dest_path = dir.path().join(".invoice/data/ATT001/test_file.txt");
        assert!(dest_path.exists());
        let content = std::fs::read(&dest_path).unwrap();
        assert_eq!(content, b"hello world attachment");

        let mut hasher = Sha256::new();
        hasher.update(b"hello world attachment");
        let expected_hash = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();
        assert_eq!(atts[0].file_hash, expected_hash);
    }

    #[test]
    fn test_add_attachment_file_not_found() {
        let _lock = CWD_MUTEX.lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("invoice.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        db::init_schema(&conn).unwrap();

        let inv = models::Invoice {
            number: "ATT002".to_string(),
            date: "2026-05-01".to_string(),
            ..Default::default()
        };
        let id = db::insert_invoice(&conn, &inv).unwrap();

        let orig_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = add_attachment(&conn, id, "ATT002", "/nonexistent/file.pdf");
        std::env::set_current_dir(&orig_dir).unwrap();
        assert!(result.is_err());
    }
}
