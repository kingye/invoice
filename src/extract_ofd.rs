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
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn create_ofd_zip(xml_content: &str, xml_path: &str) -> Vec<u8> {
        let buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file(xml_path, options).unwrap();
        zip.write_all(xml_content.as_bytes()).unwrap();
        let buf = zip.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn test_extract_from_ofd_with_original_invoice_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <EInvoice>
            <TaxSupervisionInfo>
                <InvoiceNumber>98765432109876543210</InvoiceNumber>
                <IssueTime>2026-03-20</IssueTime>
            </TaxSupervisionInfo>
            <EInvoiceData>
                <BasicInformation>
                    <TotalAmWithoutTax>500.00</TotalAmWithoutTax>
                    <TotalTaxAm>30.00</TotalTaxAm>
                    <TotalTax-includedAmount>530.00</TotalTax-includedAmount>
                </BasicInformation>
                <SellerInformation>
                    <SellerName>OFD测试公司</SellerName>
                </SellerInformation>
            </EInvoiceData>
        </EInvoice>"#;
        let data = create_ofd_zip(xml, "Doc_0/Attach/original_invoice.xml");
        let inv = extract_from_ofd(&data).unwrap();
        assert_eq!(inv.number, "98765432109876543210");
        assert_eq!(inv.date, "2026-03-20");
        assert_eq!(inv.amount, 500.0);
        assert_eq!(inv.seller_name, "OFD测试公司");
    }

    #[test]
    fn test_extract_from_ofd_with_content_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <EInvoice>
            <TaxSupervisionInfo>
                <InvoiceNumber>11111222223333344444</InvoiceNumber>
            </TaxSupervisionInfo>
        </EInvoice>"#;
        let data = create_ofd_zip(xml, "Pages/Page_0/Content.xml");
        let inv = extract_from_ofd(&data).unwrap();
        assert_eq!(inv.number, "11111222223333344444");
    }

    #[test]
    fn test_extract_from_ofd_no_xml() {
        let buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("readme.txt", options).unwrap();
        zip.write_all(b"no xml here").unwrap();
        let buf = zip.finish().unwrap();
        let data = buf.into_inner();
        let result = extract_from_ofd(&data);
        assert!(result.is_err());
    }
}
