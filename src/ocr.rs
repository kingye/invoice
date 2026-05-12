use std::fs;
use std::io::Write;
use std::path::PathBuf;

use pdf_oxide::ocr::{OcrConfig, OcrEngine};

const OCR_MODEL_DIR_NAME: &str = ".invoice/ocr";

const MODEL_FILES: [&str; 3] = ["det.onnx", "rec.onnx", "dict.txt"];

const DET_MODEL_URL: &str =
    "https://github.com/kingye/invoice/releases/download/v0.2.0-models/det.onnx";
const REC_MODEL_URL: &str =
    "https://github.com/kingye/invoice/releases/download/v0.2.0-models/rec.onnx";
const DICT_URL: &str = "https://github.com/kingye/invoice/releases/download/v0.2.0-models/dict.txt";

pub fn ocr_model_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("INVOICE_OCR_MODEL_DIR") {
        PathBuf::from(dir)
    } else {
        let home = dirs_home();
        home.join(OCR_MODEL_DIR_NAME)
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn model_files_exist() -> bool {
    let dir = ocr_model_dir();
    MODEL_FILES.iter().all(|f| dir.join(f).exists())
}

pub fn download_models() -> Result<(), Box<dyn std::error::Error>> {
    let dir = ocr_model_dir();
    fs::create_dir_all(&dir)?;

    let det_path = dir.join("det.onnx");
    let rec_path = dir.join("rec.onnx");
    let dict_path = dir.join("dict.txt");

    if !det_path.exists() {
        println!("Downloading detection model...");
        download_file_atomic(DET_MODEL_URL, &det_path)?;
        println!("  Detection model saved to {}", det_path.display());
    } else {
        println!("Detection model already exists, skipping.");
    }

    if !rec_path.exists() {
        println!("Downloading recognition model...");
        download_file_atomic(REC_MODEL_URL, &rec_path)?;
        println!("  Recognition model saved to {}", rec_path.display());
    } else {
        println!("Recognition model already exists, skipping.");
    }

    if !dict_path.exists() {
        println!("Downloading character dictionary...");
        download_file_atomic(DICT_URL, &dict_path)?;
        println!("  Dictionary saved to {}", dict_path.display());
    } else {
        println!("Dictionary already exists, skipping.");
    }

    Ok(())
}

fn download_file_atomic(url: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let tls_config = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls_config)
        .build()
        .into();

    let tmp_path = path.with_extension("tmp");

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let response = agent.get(url).call()?;
        let mut reader = response.into_body().into_reader();
        let mut file = fs::File::create(&tmp_path)?;
        std::io::copy(&mut reader, &mut file)?;
        file.flush()?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            fs::rename(&tmp_path, path)?;
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

pub fn get_ocr_engine() -> Result<OcrEngine, Box<dyn std::error::Error>> {
    let dir = ocr_model_dir();
    get_ocr_engine_with_dir(dir.to_str().unwrap_or("/tmp"))
}

pub fn get_ocr_engine_with_dir(model_dir: &str) -> Result<OcrEngine, Box<dyn std::error::Error>> {
    let dir = PathBuf::from(model_dir);
    let config = OcrConfig::default();
    let engine = OcrEngine::new(
        dir.join("det.onnx"),
        dir.join("rec.onnx"),
        dir.join("dict.txt"),
        config,
    )?;
    Ok(engine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_ocr_model_dir_env_override() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("INVOICE_OCR_MODEL_DIR").ok();
        std::env::set_var("INVOICE_OCR_MODEL_DIR", "/tmp/test_ocr_models");
        let dir = ocr_model_dir();
        assert_eq!(dir, PathBuf::from("/tmp/test_ocr_models"));
        match prev {
            Some(v) => std::env::set_var("INVOICE_OCR_MODEL_DIR", v),
            None => std::env::remove_var("INVOICE_OCR_MODEL_DIR"),
        }
    }

    #[test]
    fn test_model_files_exist_missing() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("INVOICE_OCR_MODEL_DIR").ok();
        std::env::set_var(
            "INVOICE_OCR_MODEL_DIR",
            "/tmp/nonexistent_ocr_test_dir_12345",
        );
        assert!(!model_files_exist());
        match prev {
            Some(v) => std::env::set_var("INVOICE_OCR_MODEL_DIR", v),
            None => std::env::remove_var("INVOICE_OCR_MODEL_DIR"),
        }
    }

    #[test]
    fn test_download_models_skip_existing() {
        let _lock = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("invoice_ocr_test_existing");
        fs::create_dir_all(&dir).ok();
        fs::write(dir.join("det.onnx"), "fake").ok();
        fs::write(dir.join("rec.onnx"), "fake").ok();
        fs::write(dir.join("dict.txt"), "fake").ok();

        let prev = std::env::var("INVOICE_OCR_MODEL_DIR").ok();
        std::env::set_var("INVOICE_OCR_MODEL_DIR", dir.to_str().unwrap());

        let result = download_models();
        assert!(result.is_ok());

        match prev {
            Some(v) => std::env::set_var("INVOICE_OCR_MODEL_DIR", v),
            None => std::env::remove_var("INVOICE_OCR_MODEL_DIR"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_download_file_atomic_cleanup_on_error() {
        let dir = std::env::temp_dir().join("invoice_ocr_test_atomic");
        fs::create_dir_all(&dir).ok();
        let target = dir.join("should_not_exist.onnx");

        let result = download_file_atomic("http://invalid.host.invalid/file", &target);
        assert!(result.is_err());
        assert!(!target.exists());
        assert!(!dir.join("should_not_exist.tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dirs_home_fallback() {
        let home = dirs_home();
        assert_ne!(home, PathBuf::from("."));
    }
}
