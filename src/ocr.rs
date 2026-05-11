use std::fs;
use std::path::PathBuf;

use pdf_oxide::ocr::{OcrConfig, OcrEngine};

const OCR_MODEL_DIR_NAME: &str = ".invoice/ocr";

const MODEL_FILES: [&str; 3] = ["det.onnx", "rec.onnx", "dict.txt"];

const DET_MODEL_URL: &str =
    "https://paddleocr.bj.bcebos.com/PP-OCRv4/chinese/ch_PP-OCRv4_det_infer.tar";
const REC_MODEL_URL: &str =
    "https://paddleocr.bj.bcebos.com/PP-OCRv4/chinese/ch_PP-OCRv4_rec_infer.tar";
const DICT_URL: &str =
    "https://raw.githubusercontent.com/PaddlePaddle/PaddleOCR/main/ppocr/utils/ppocr_keys_v1.txt";

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
        .unwrap_or_else(|_| PathBuf::from("."))
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
        download_file(DET_MODEL_URL, &det_path)?;
        println!("  Detection model saved to {}", det_path.display());
    } else {
        println!("Detection model already exists, skipping.");
    }

    if !rec_path.exists() {
        println!("Downloading recognition model...");
        download_file(REC_MODEL_URL, &rec_path)?;
        println!("  Recognition model saved to {}", rec_path.display());
    } else {
        println!("Recognition model already exists, skipping.");
    }

    if !dict_path.exists() {
        println!("Downloading character dictionary...");
        download_file(DICT_URL, &dict_path)?;
        println!("  Dictionary saved to {}", dict_path.display());
    } else {
        println!("Dictionary already exists, skipping.");
    }

    Ok(())
}

fn download_file(url: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;

    let mut reader = response.into_body().into_reader();
    let mut file = fs::File::create(path)?;
    std::io::copy(&mut reader, &mut file)?;
    Ok(())
}

pub fn get_ocr_engine() -> Result<OcrEngine, Box<dyn std::error::Error>> {
    let dir = ocr_model_dir();
    get_ocr_engine_with_dir(dir.to_str().unwrap_or("."))
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

    #[test]
    fn test_ocr_model_dir_env_override() {
        std::env::set_var("INVOICE_OCR_MODEL_DIR", "/tmp/test_ocr_models");
        let dir = ocr_model_dir();
        assert_eq!(dir, PathBuf::from("/tmp/test_ocr_models"));
        std::env::remove_var("INVOICE_OCR_MODEL_DIR");
    }

    #[test]
    fn test_model_files_exist_missing() {
        std::env::set_var("INVOICE_OCR_MODEL_DIR", "/tmp/nonexistent_ocr_test_dir");
        assert!(!model_files_exist());
        std::env::remove_var("INVOICE_OCR_MODEL_DIR");
    }

    #[test]
    fn test_download_models_skip_existing() {
        let dir = std::env::temp_dir().join("invoice_ocr_test_existing");
        fs::create_dir_all(&dir).ok();
        fs::write(dir.join("det.onnx"), "fake").ok();
        fs::write(dir.join("rec.onnx"), "fake").ok();
        fs::write(dir.join("dict.txt"), "fake").ok();

        std::env::set_var("INVOICE_OCR_MODEL_DIR", dir.to_str().unwrap());

        let result = download_models();
        assert!(result.is_ok());

        std::env::remove_var("INVOICE_OCR_MODEL_DIR");
        let _ = fs::remove_dir_all(&dir);
    }
}
