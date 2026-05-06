use crate::extract_xml;
use crate::models;
use std::io::Read;

pub fn extract_from_ofd(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut xml_content: Option<String> = None;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.contains("original_invoice.xml")
            || name.contains("Attach") && name.ends_with(".xml")
        {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            xml_content = Some(content);
            break;
        }
    }

    if xml_content.is_none() {
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            if name.contains("Content") && name.ends_with(".xml") {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                xml_content = Some(content);
                break;
            }
        }
    }

    if let Some(content) = xml_content {
        extract_xml::extract_from_xml(&content)
    } else {
        Err("No invoice XML found in OFD file".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_extract_from_ofd_placeholder() {
        assert!(true);
    }
}
