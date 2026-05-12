use rmcp::handler::server::wrapper::Parameters;
use serde::Deserialize;
use schemars::JsonSchema;

use crate::closing;
use crate::models;
use crate::ops;

#[derive(Debug, Clone)]
pub struct InvoiceMcp;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddInvoiceParams {
    pub number: String,
    pub date: String,
    #[serde(default)]
    pub inv_type: String,
    #[serde(default)]
    pub item_name: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub tax_rate: f64,
    #[serde(default)]
    pub tax: f64,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub seller_name: String,
    #[serde(default)]
    pub seller_tax_id: String,
    #[serde(default)]
    pub buyer_name: String,
    #[serde(default)]
    pub buyer_tax_id: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub attach: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListInvoicesParams {
    pub month: Option<String>,
    pub year: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowInvoiceParams {
    pub id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditInvoiceParams {
    pub id: i64,
    pub number: Option<String>,
    pub date: Option<String>,
    pub inv_type: Option<String>,
    pub item_name: Option<String>,
    pub amount: Option<f64>,
    pub tax_rate: Option<f64>,
    pub tax: Option<f64>,
    pub total: Option<f64>,
    pub seller_name: Option<String>,
    pub seller_tax_id: Option<String>,
    pub buyer_name: Option<String>,
    pub buyer_tax_id: Option<String>,
    pub category: Option<String>,
    pub remark: Option<String>,
    #[serde(default)]
    pub attach: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteInvoiceParams {
    pub id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportInvoiceParams {
    #[schemars(description = "PDF 发票文件路径")]
    pub path: String,
    pub category: Option<String>,
    pub remark: Option<String>,
    pub ocr_model_dir: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClosePeriodParams {
    pub month: Option<String>,
    pub year: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportReportsParams {
    pub month: Option<String>,
    pub year: Option<String>,
    #[serde(default = "default_output_dir")]
    pub output: String,
}

fn default_output_dir() -> String {
    ".".to_string()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InitParams {}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

async fn blocking<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("spawn_blocking error: {}", e))
        .and_then(|r| r)
}

#[rmcp::tool_router(server_handler)]
impl InvoiceMcp {
    #[rmcp::tool(description = "Initialize the invoice database. Downloads OCR models if needed.")]
    async fn invoice_init(&self, _params: Parameters<InitParams>) -> String {
        match blocking(|| ops::init_database().map_err(|e| e.to_string())).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "Add a new invoice manually with all fields")]
    async fn invoice_add(&self, Parameters(params): Parameters<AddInvoiceParams>) -> String {
        match blocking(move || {
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            let id = ops::add_invoice(
                &conn,
                &params.number,
                &params.date,
                &params.inv_type,
                &params.item_name,
                params.amount,
                params.tax_rate,
                params.tax,
                params.total,
                &params.seller_name,
                &params.seller_tax_id,
                &params.buyer_name,
                &params.buyer_tax_id,
                &params.category,
                &params.remark,
                &params.attach,
            )
            .map_err(|e| e.to_string())?;
            Ok(format!("Invoice added with id={}", id))
        })
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "List invoices with optional filters (month, year, category)")]
    async fn invoice_list(&self, Parameters(params): Parameters<ListInvoicesParams>) -> String {
        match blocking(move || {
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            let invoices = ops::list_invoices(
                &conn,
                params.month.as_deref(),
                params.year.as_deref(),
                params.category.as_deref(),
            )
            .map_err(|e| e.to_string())?;
            Ok(to_json(&invoices))
        })
        .await
        {
            Ok(json) => json,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "Show detailed information of a single invoice by ID")]
    async fn invoice_show(&self, Parameters(params): Parameters<ShowInvoiceParams>) -> String {
        let id = params.id;
        match blocking(move || {
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            match ops::get_invoice(&conn, id).map_err(|e| e.to_string())? {
                Some(inv) => Ok(to_json(&inv)),
                None => Err(format!("Invoice #{} not found", id)),
            }
        })
        .await
        {
            Ok(json) => json,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "Edit an existing invoice's fields. Only provided fields are updated.")]
    async fn invoice_edit(&self, Parameters(params): Parameters<EditInvoiceParams>) -> String {
        match blocking(move || {
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            let changed = ops::edit_invoice(
                &conn,
                params.id,
                params.number,
                params.date,
                params.inv_type,
                params.item_name,
                params.amount,
                params.tax_rate,
                params.tax,
                params.total,
                params.seller_name,
                params.seller_tax_id,
                params.buyer_name,
                params.buyer_tax_id,
                params.category,
                params.remark,
                &params.attach,
            )
            .map_err(|e| e.to_string())?;
            if changed > 0 {
                Ok(format!("Invoice #{} updated", params.id))
            } else {
                Err(format!("Invoice #{} not found", params.id))
            }
        })
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "Delete an invoice by ID. Cannot delete invoices in closed periods.")]
    async fn invoice_delete(&self, Parameters(params): Parameters<DeleteInvoiceParams>) -> String {
        let id = params.id;
        match blocking(move || {
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            let changed = ops::delete_invoice(&conn, id).map_err(|e| e.to_string())?;
            if changed > 0 {
                Ok(format!("Invoice #{} deleted", id))
            } else {
                Err(format!("Invoice #{} not found", id))
            }
        })
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "从 PDF 文件导入发票，自动提取发票号码、日期、金额、税率、买卖方等信息")]
    async fn invoice_import(&self, Parameters(params): Parameters<ImportInvoiceParams>) -> String {
        match blocking(move || {
            let inv = ops::import_invoice(
                &params.path,
                params.category.as_deref(),
                params.remark.as_deref(),
                params.ocr_model_dir.as_deref(),
            )
            .map_err(|e| e.to_string())?;
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            let id = ops::insert_imported_invoice(&conn, &inv, &params.path)
                .map_err(|e| e.to_string())?;
            Ok(format!("Invoice imported with id={}", id))
        })
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "Close an accounting period (month or year). Generates reports and archives data.")]
    async fn invoice_close(&self, Parameters(params): Parameters<ClosePeriodParams>) -> String {
        match blocking(move || {
            let (close_type, period) = match (&params.month, &params.year) {
                (Some(m), _) => (closing::CloseType::Month, m.clone()),
                (_, Some(y)) => (closing::CloseType::Year, y.clone()),
                _ => return Err("Usage: provide --month YYYY-MM or --year YYYY".to_string()),
            };
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            ops::close_period(&conn, close_type, &period)
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "Period {} closed successfully. Archive: .invoice/close_{}.zip",
                period, period
            ))
        })
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[rmcp::tool(description = "Export invoice reports (detail and summary) for a given period")]
    async fn invoice_export(&self, Parameters(params): Parameters<ExportReportsParams>) -> String {
        match blocking(move || {
            let (close_type, period) = match (&params.month, &params.year) {
                (Some(m), _) => (closing::CloseType::Month, m.clone()),
                (_, Some(y)) => (closing::CloseType::Year, y.clone()),
                _ => return Err("Usage: provide --month YYYY-MM or --year YYYY".to_string()),
            };
            let conn = ops::open_db().map_err(|e| e.to_string())?;
            let (_invoices, detail_path, summary_path) =
                ops::export_reports(&conn, close_type, &period, &params.output)
                    .map_err(|e| e.to_string())?;
            let result = models::ExportResult {
                detail_path,
                summary_path,
                output_dir: params.output.clone(),
                period: period.clone(),
            };
            Ok(to_json(&result))
        })
        .await
        {
            Ok(json) => json,
            Err(e) => format!("Error: {}", e),
        }
    }
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let server = InvoiceMcp;
    let transport = rmcp::transport::io::stdio();
    rmcp::serve_server(server, transport).await?;
    Ok(())
}
