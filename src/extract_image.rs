use crate::models;

pub(crate) fn extract_from_image(
    data: &[u8],
    ocr_model_dir: Option<&str>,
) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(data)?;

    let ocr_output = if let Some(model_dir) = ocr_model_dir {
        let engine = crate::ocr::get_ocr_engine_with_dir(model_dir)?;
        engine.ocr_image(&img)?
    } else {
        let engine = crate::ocr::get_ocr_engine()?;
        engine.ocr_image(&img)?
    };

    let text = ocr_output.text_in_reading_order();
    let mut inv = models::Invoice::default();
    crate::extract_pdf::parse_invoice_text(&text, &mut inv);

    Ok(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_image_invalid_data() {
        // Garbage bytes that are not a valid image format
        let result = extract_from_image(&[0x00, 0x01, 0x02, 0x03], None);
        // image::load_from_memory should fail on invalid data
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_from_image_missing_model() {
        // Minimal valid PNG (1x1 red pixel) — decodes successfully but
        // OCR with a non-existent model directory should fail
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, // IHDR CRC
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00,
            0x02, 0x02, 0x01, 0x00, 0x1C, 0xCC, 0x6D, 0xE4, // IDAT CRC
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ];
        let result = extract_from_image(png, Some("/tmp/nonexistent_ocr_model_test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_from_image_valid_png_no_model_files() {
        // Minimal valid PNG with no model files available — OCR will fail
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE,
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54,
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00,
            0x02, 0x02, 0x01, 0x00, 0x1C, 0xCC, 0x6D, 0xE4,
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
            0xAE, 0x42, 0x60, 0x82,
        ];
        // Without model files present on disk, get_ocr_engine() will fail
        let result = extract_from_image(png, None);
        // OCR engine init may fail if model files are missing
        assert!(result.is_err());
    }
}
