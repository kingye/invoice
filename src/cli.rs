use clap::{Parser, Subcommand};

use crate::closing;
use crate::ops;

#[derive(Parser)]
#[command(name = "invoice", version = "0.1.0", about = "轻量级命令行记账系统")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Add {
        #[arg(long)]
        number: String,
        #[arg(long)]
        date: String,
        #[arg(long, default_value = "")]
        r#type: String,
        #[arg(long, default_value = "")]
        item: String,
        #[arg(long, default_value_t = 0.0)]
        amount: f64,
        #[arg(long, default_value_t = 0.0)]
        tax_rate: f64,
        #[arg(long, default_value_t = 0.0)]
        tax: f64,
        #[arg(long, default_value_t = 0.0)]
        total: f64,
        #[arg(long, default_value = "")]
        seller: String,
        #[arg(long, default_value = "")]
        seller_tax: String,
        #[arg(long, default_value = "")]
        buyer: String,
        #[arg(long, default_value = "")]
        buyer_tax: String,
        #[arg(long, default_value = "")]
        category: String,
        #[arg(long, default_value = "")]
        remark: String,
        #[arg(long)]
        attach: Vec<String>,
    },
    List {
        #[arg(long)]
        month: Option<String>,
        #[arg(long)]
        year: Option<String>,
        #[arg(long)]
        category: Option<String>,
    },
    Show {
        id: i64,
    },
    Edit {
        id: i64,
        #[arg(long)]
        number: Option<String>,
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        amount: Option<f64>,
        #[arg(long)]
        tax_rate: Option<f64>,
        #[arg(long)]
        tax: Option<f64>,
        #[arg(long)]
        total: Option<f64>,
        #[arg(long)]
        seller: Option<String>,
        #[arg(long)]
        seller_tax: Option<String>,
        #[arg(long)]
        buyer: Option<String>,
        #[arg(long)]
        buyer_tax: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        remark: Option<String>,
        #[arg(long)]
        attach: Vec<String>,
    },
    Delete {
        id: i64,
    },
    Close {
        #[arg(long)]
        month: Option<String>,
        #[arg(long)]
        year: Option<String>,
    },
    Export {
        #[arg(long)]
        month: Option<String>,
        #[arg(long)]
        year: Option<String>,
        #[arg(long, default_value = ".")]
        output: String,
    },
    Import {
        path: String,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        remark: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        ocr_model_dir: Option<String>,
    },
    Mcp,
}

pub fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Add {
            number,
            date,
            r#type,
            item,
            amount,
            tax_rate,
            tax,
            total,
            seller,
            seller_tax,
            buyer,
            buyer_tax,
            category,
            remark,
            attach,
        } => cmd_add(
            &number,
            &date,
            &r#type,
            &item,
            amount,
            tax_rate,
            tax,
            total,
            &seller,
            &seller_tax,
            &buyer,
            &buyer_tax,
            &category,
            &remark,
            &attach,
        ),
        Commands::List {
            month,
            year,
            category,
        } => cmd_list(month.as_deref(), year.as_deref(), category.as_deref()),
        Commands::Show { id } => cmd_show(id),
        Commands::Edit {
            id,
            number,
            date,
            r#type,
            item,
            amount,
            tax_rate,
            tax,
            total,
            seller,
            seller_tax,
            buyer,
            buyer_tax,
            category,
            remark,
            attach,
        } => cmd_edit(
            id, number, date, r#type, item, amount, tax_rate, tax, total, seller, seller_tax,
            buyer, buyer_tax, category, remark, &attach,
        ),
        Commands::Delete { id } => cmd_delete(id),
        Commands::Close { month, year } => cmd_close(month.as_deref(), year.as_deref()),
        Commands::Export {
            month,
            year,
            output,
        } => cmd_export(month.as_deref(), year.as_deref(), &output),
        Commands::Import {
            path,
            category,
            remark,
            dry_run,
            ocr_model_dir,
        } => cmd_import(
            &path,
            category.as_deref(),
            remark.as_deref(),
            dry_run,
            ocr_model_dir.as_deref(),
        ),
        Commands::Mcp => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(crate::mcp::run_server())
        }
    }
}

fn cmd_init() -> Result<(), Box<dyn std::error::Error>> {
    let result = ops::init_database()?;
    println!("{}", result);
    Ok(())
}

fn cmd_add(
    number: &str,
    date: &str,
    inv_type: &str,
    item_name: &str,
    amount: f64,
    tax_rate: f64,
    tax: f64,
    total: f64,
    seller_name: &str,
    seller_tax_id: &str,
    buyer_name: &str,
    buyer_tax_id: &str,
    category: &str,
    remark: &str,
    attach_paths: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = ops::open_db()?;
    let id = ops::add_invoice(
        &conn,
        number,
        date,
        inv_type,
        item_name,
        amount,
        tax_rate,
        tax,
        total,
        seller_name,
        seller_tax_id,
        buyer_name,
        buyer_tax_id,
        category,
        remark,
        attach_paths,
    )?;
    println!("Invoice added with id={}", id);
    Ok(())
}

fn cmd_list(
    month: Option<&str>,
    year: Option<&str>,
    category: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = ops::open_db()?;
    let invoices = ops::list_invoices(&conn, month, year, category)?;
    println!(
        "{:>4}  {:<20} {:<12} {:<10} {:<12} {:>10} {:>6} {:>8} {:>10} {:<16}",
        "ID", "Number", "Date", "Type", "Item", "Amount", "Tax%", "Tax", "Total", "Seller"
    );
    println!("{}", "-".repeat(120));
    for inv in &invoices {
        println!(
            "{:>4}  {:<20} {:<12} {:<10} {:<12} {:>10.2} {:>5.0}% {:>8.2} {:>10.2} {:<16}",
            inv.id,
            inv.number,
            inv.date,
            inv.inv_type,
            inv.item_name,
            inv.amount,
            inv.tax_rate * 100.0,
            inv.tax,
            inv.total,
            inv.seller_name
        );
    }
    Ok(())
}

fn cmd_show(id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let conn = ops::open_db()?;
    match ops::get_invoice(&conn, id)? {
        Some(inv) => {
            println!("Invoice #{}", inv.id);
            println!("  Number:       {}", inv.number);
            println!("  Date:         {}", inv.date);
            println!("  Type:         {}", inv.inv_type);
            println!("  Item:         {}", inv.item_name);
            println!("  Amount:       {:.2}", inv.amount);
            println!("  Tax Rate:     {:.0}%", inv.tax_rate * 100.0);
            println!("  Tax:          {:.2}", inv.tax);
            println!("  Total:        {:.2}", inv.total);
            println!("  Seller:       {}", inv.seller_name);
            println!("  Seller TaxID: {}", inv.seller_tax_id);
            println!("  Buyer:        {}", inv.buyer_name);
            println!("  Buyer TaxID:  {}", inv.buyer_tax_id);
            println!("  Category:     {}", inv.category);
            println!("  Remark:       {}", inv.remark);
            println!("  Created:      {}", inv.created_at);
            println!("  Updated:      {}", inv.updated_at);

            let atts = ops::get_attachments(&conn, id)?;
            println!("\n  Attachments:");
            if atts.is_empty() {
                println!("    (none)");
            } else {
                for att in &atts {
                    println!(
                        "    - {} ({} bytes) sha256:{}",
                        att.filename, att.file_size, att.file_hash
                    );
                }
            }
        }
        None => println!("Invoice #{} not found", id),
    }
    Ok(())
}

fn cmd_edit(
    id: i64,
    number: Option<String>,
    date: Option<String>,
    inv_type: Option<String>,
    item_name: Option<String>,
    amount: Option<f64>,
    tax_rate: Option<f64>,
    tax: Option<f64>,
    total: Option<f64>,
    seller_name: Option<String>,
    seller_tax_id: Option<String>,
    buyer_name: Option<String>,
    buyer_tax_id: Option<String>,
    category: Option<String>,
    remark: Option<String>,
    attach_paths: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = ops::open_db()?;
    match ops::edit_invoice(
        &conn,
        id,
        number,
        date,
        inv_type,
        item_name,
        amount,
        tax_rate,
        tax,
        total,
        seller_name,
        seller_tax_id,
        buyer_name,
        buyer_tax_id,
        category,
        remark,
        attach_paths,
    ) {
        Ok(changed) => {
            if changed > 0 {
                println!("Invoice #{} updated", id);
            } else {
                println!("Invoice #{} not found", id);
            }
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("closed period") {
                println!(
                    "Error: Invoice #{} is in a closed period and cannot be modified",
                    id
                );
                return Ok(());
            }
            return Err(e);
        }
    }
    Ok(())
}

fn cmd_delete(id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let conn = ops::open_db()?;
    match ops::delete_invoice(&conn, id) {
        Ok(changed) => {
            if changed > 0 {
                println!("Invoice #{} deleted", id);
            } else {
                println!("Invoice #{} not found", id);
            }
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("closed period") {
                println!(
                    "Error: Invoice #{} is in a closed period and cannot be deleted",
                    id
                );
                return Ok(());
            }
            return Err(e);
        }
    }
    Ok(())
}

fn cmd_close(month: Option<&str>, year: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let (close_type, period) = match (month, year) {
        (Some(m), _) => (closing::CloseType::Month, m),
        (_, Some(y)) => (closing::CloseType::Year, y),
        _ => {
            println!("Usage: invoice close --month YYYY-MM or --year YYYY");
            return Ok(());
        }
    };
    let conn = ops::open_db()?;
    match ops::close_period(&conn, close_type, period) {
        Ok(()) => println!(
            "Period {} closed successfully. Archive: .invoice/close_{}.zip",
            period, period
        ),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already closed") {
                println!("Error: Period {} is already closed", period);
            } else if msg.contains("No invoices") {
                println!("Error: No invoices found for period {}", period);
            } else {
                return Err(e);
            }
        }
    }
    Ok(())
}

fn cmd_export(
    month: Option<&str>,
    year: Option<&str>,
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (close_type, period) = match (month, year) {
        (Some(m), _) => (closing::CloseType::Month, m),
        (_, Some(y)) => (closing::CloseType::Year, y),
        _ => {
            println!("Usage: invoice export --month YYYY-MM or --year YYYY [--output DIR]");
            return Ok(());
        }
    };
    let conn = ops::open_db()?;
    match ops::export_reports(&conn, close_type, period, output_dir) {
        Ok((_invoices, detail_path, summary_path)) => {
            println!("Reports exported to {}/", output_dir);
            println!(
                "  Detail: {}",
                std::path::Path::new(&detail_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&detail_path)
            );
            println!(
                "  Summary: {}",
                std::path::Path::new(&summary_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&summary_path)
            );
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("No invoices") {
                println!("No invoices found for period {}", period);
                return Ok(());
            }
            return Err(e);
        }
    }
    Ok(())
}

fn cmd_import(
    path: &str,
    category: Option<&str>,
    remark: Option<&str>,
    dry_run: bool,
    ocr_model_dir: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let inv = ops::import_invoice(path, category, remark, ocr_model_dir)?;
    if dry_run {
        let json = serde_json::to_string_pretty(&inv)?;
        println!("{}", json);
        return Ok(());
    }
    let conn = ops::open_db()?;
    let id = ops::insert_imported_invoice(&conn, &inv, path)?;
    println!("Invoice imported with id={}", id);
    Ok(())
}
