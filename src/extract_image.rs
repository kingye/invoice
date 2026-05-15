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

    let text = ocr_output.text();
    let mut inv = models::Invoice::default();
    crate::extract_pdf::parse_invoice_text(&text, &mut inv);

    Ok(inv)
}
