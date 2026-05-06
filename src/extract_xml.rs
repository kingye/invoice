use crate::models;
use std::io::Read;

pub fn extract_from_xml(xml_content: &str) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let doc = roxmltree::Document::parse(xml_content)?;

    let mut inv = models::Invoice::default();

    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("InvoiceNumber")) {
        inv.number = node.text().unwrap_or("").to_string();
    }
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("IssueTime")) {
        inv.date = node.text().unwrap_or("").to_string();
    }

    let label_name = doc
        .descendants()
        .find(|n| n.has_tag_name("LabelName"))
        .and_then(|n| n.text())
        .unwrap_or("")
        .to_string();
    let einv_type = doc
        .descendants()
        .find(|n| n.has_tag_name("EInvoiceType"))
        .and_then(|n| n.text())
        .unwrap_or("");
    if !label_name.is_empty() {
        inv.inv_type = format!("电子发票{}", label_name);
    } else if !einv_type.is_empty() {
        inv.inv_type = einv_type.to_string();
    }

    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("ItemName")) {
        let item_text = node.text().unwrap_or("");
        if let Some(pos) = item_text.find('*') {
            if let Some(end) = item_text[pos + 1..].find('*') {
                inv.category = item_text[pos + 1..pos + 1 + end].to_string();
                inv.item_name = item_text[pos + 1 + end + 1..].to_string();
            } else {
                inv.item_name = item_text.to_string();
            }
        } else {
            inv.item_name = item_text.to_string();
        }
    }

    if let Some(node) = doc
        .descendants()
        .find(|n| n.has_tag_name("TotalAmWithoutTax"))
    {
        inv.amount = node.text().unwrap_or("0").parse().unwrap_or(0.0);
    }
    for node in doc.descendants().filter(|n| n.has_tag_name("TaxRate")) {
        let rate: f64 = node.text().unwrap_or("0").parse().unwrap_or(0.0);
        if rate > 0.0 {
            inv.tax_rate = rate;
            break;
        }
    }
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("TotalTaxAm")) {
        inv.tax = node.text().unwrap_or("0").parse().unwrap_or(0.0);
    }
    if let Some(node) = doc
        .descendants()
        .find(|n| n.has_tag_name("TotalTax-includedAmount"))
    {
        inv.total = node.text().unwrap_or("0").parse().unwrap_or(0.0);
    }
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("SellerName")) {
        inv.seller_name = node.text().unwrap_or("").to_string();
    }
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("SellerIdNum")) {
        inv.seller_tax_id = node.text().unwrap_or("").to_string();
    }
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("BuyerName")) {
        inv.buyer_name = node.text().unwrap_or("").to_string();
    }
    if let Some(node) = doc.descendants().find(|n| n.has_tag_name("BuyerIdNum")) {
        inv.buyer_tax_id = node.text().unwrap_or("").to_string();
    }

    Ok(inv)
}

pub fn extract_from_xml_in_zip(data: &[u8]) -> Result<models::Invoice, Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.ends_with(".xml") {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let inv = extract_from_xml(&content);
            if inv.is_ok() {
                return inv;
            }
        }
    }
    Err("No valid XML invoice found in ZIP".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_xml_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <EInvoice>
            <TaxSupervisionInfo>
                <InvoiceNumber>12345678901234567890</InvoiceNumber>
                <IssueTime>2026-04-15</IssueTime>
            </TaxSupervisionInfo>
            <Header>
                <InherentLabel>
                    <EInvoiceType>
                        <LabelName>普通发票</LabelName>
                    </EInvoiceType>
                </InherentLabel>
            </Header>
            <EInvoiceData>
                <BasicInformation>
                    <TotalAmWithoutTax>1000.00</TotalAmWithoutTax>
                    <TotalTaxAm>60.00</TotalTaxAm>
                    <TotalTax-includedAmount>1060.00</TotalTax-includedAmount>
                </BasicInformation>
                <IssuItemInformation>
                    <ItemName>*服务*技术咨询</ItemName>
                    <TaxRate>0.06</TaxRate>
                </IssuItemInformation>
                <SellerInformation>
                    <SellerName>测试公司</SellerName>
                    <SellerIdNum>91110000MA01</SellerIdNum>
                </SellerInformation>
                <BuyerInformation>
                    <BuyerName>购买方公司</BuyerName>
                    <BuyerIdNum>91310000MB01</BuyerIdNum>
                </BuyerInformation>
            </EInvoiceData>
        </EInvoice>"#;

        let inv = extract_from_xml(xml).unwrap();
        assert_eq!(inv.number, "12345678901234567890");
        assert_eq!(inv.date, "2026-04-15");
        assert_eq!(inv.inv_type, "电子发票普通发票");
        assert_eq!(inv.item_name, "技术咨询");
        assert_eq!(inv.category, "服务");
        assert_eq!(inv.amount, 1000.0);
        assert_eq!(inv.tax_rate, 0.06);
        assert_eq!(inv.tax, 60.0);
        assert_eq!(inv.total, 1060.0);
        assert_eq!(inv.seller_name, "测试公司");
        assert_eq!(inv.seller_tax_id, "91110000MA01");
        assert_eq!(inv.buyer_name, "购买方公司");
        assert_eq!(inv.buyer_tax_id, "91310000MB01");
    }
}
