use clap::{Parser, Subcommand};

use crate::attachment;
use crate::closing;
use crate::db;
use crate::import;
use crate::models;
use crate::report;

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
    },
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
        } => cmd_import(&path, category.as_deref(), remark.as_deref(), dry_run),
    }
}

fn open_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
    let conn = db::init_db()?;
    db::init_schema(&conn)?;
    Ok(conn)
}

fn cmd_init() -> Result<(), Box<dyn std::error::Error>> {
    let conn = db::init_db()?;
    db::init_schema(&conn)?;
    let cwd = std::env::current_dir()?;
    println!(
        "Initialized invoice database in {}/.invoice/invoice.db",
        cwd.display()
    );
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
    let conn = open_db()?;
    let inv = models::Invoice {
        number: number.to_string(),
        date: date.to_string(),
        inv_type: inv_type.to_string(),
        item_name: item_name.to_string(),
        amount,
        tax_rate,
        tax,
        total,
        seller_name: seller_name.to_string(),
        seller_tax_id: seller_tax_id.to_string(),
        buyer_name: buyer_name.to_string(),
        buyer_tax_id: buyer_tax_id.to_string(),
        category: category.to_string(),
        remark: remark.to_string(),
        ..Default::default()
    };
    let id = db::insert_invoice(&conn, &inv)?;
    for att_path in attach_paths {
        if let Err(e) = attachment::add_attachment(&conn, id, number, att_path) {
            eprintln!("Failed to add attachment '{}': {}", att_path, e);
        }
    }
    println!("Invoice added with id={}", id);
    Ok(())
}

fn cmd_list(
    month: Option<&str>,
    year: Option<&str>,
    category: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db()?;
    let invoices = db::query_invoices(&conn, month, year, category)?;
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
    let conn = open_db()?;
    match db::get_invoice(&conn, id)? {
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

            let atts = db::get_attachments_for_invoice(&conn, id)?;
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
    let conn = open_db()?;

    if closing::check_invoice_closed(&conn, id)? {
        println!(
            "Error: Invoice #{} is in a closed period and cannot be modified",
            id
        );
        return Ok(());
    }

    let updates = models::InvoiceUpdate {
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
    };
    let changed = db::update_invoice(&conn, id, &updates)?;
    if changed > 0 {
        println!("Invoice #{} updated", id);
    } else {
        println!("Invoice #{} not found", id);
    }

    for att_path in attach_paths {
        if let Ok(inv_number) = db::get_invoice_number(&conn, id) {
            match attachment::add_attachment(&conn, id, &inv_number, att_path) {
                Ok(()) => println!("  Attachment added: {}", att_path),
                Err(e) => eprintln!("Failed to add attachment '{}': {}", att_path, e),
            }
        }
    }
    Ok(())
}

fn cmd_delete(id: i64) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db()?;
    if closing::check_invoice_closed(&conn, id)? {
        println!(
            "Error: Invoice #{} is in a closed period and cannot be deleted",
            id
        );
        return Ok(());
    }
    let changed = db::delete_invoice(&conn, id)?;
    if changed > 0 {
        println!("Invoice #{} deleted", id);
    } else {
        println!("Invoice #{} not found", id);
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
    let conn = open_db()?;
    match closing::close_period(&conn, close_type, period) {
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
    let conn = open_db()?;
    let invoices = closing::query_invoices_for_period(&conn, close_type, period)?;
    if invoices.is_empty() {
        println!("No invoices found for period {}", period);
        return Ok(());
    }
    std::fs::create_dir_all(output_dir)?;
    let detail_path = format!("{}/明细表_{}.xlsx", output_dir, period);
    let summary_path = format!("{}/汇总表_{}.xlsx", output_dir, period);
    report::generate_detail_report(&invoices, &detail_path)?;
    let summary_entries = report::compute_summary(&invoices);
    report::generate_summary_report(&summary_entries, &summary_path)?;
    println!("Reports exported to {}/", output_dir);
    println!("  Detail: 明细表_{}.xlsx", period);
    println!("  Summary: 汇总表_{}.xlsx", period);
    Ok(())
}

fn cmd_import(
    path: &str,
    category: Option<&str>,
    remark: Option<&str>,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let extracted = import::extract_invoice(path)?;
    let mut inv = extracted;
    if let Some(cat) = category {
        inv.category = cat.to_string();
    }
    if let Some(rem) = remark {
        inv.remark = rem.to_string();
    }
    if dry_run {
        let json = serde_json::to_string_pretty(&inv)?;
        println!("{}", json);
        return Ok(());
    }
    let conn = open_db()?;
    let id = db::insert_invoice(&conn, &inv)?;
    if let Err(e) = attachment::add_attachment(&conn, id, &inv.number, path) {
        eprintln!("Warning: Failed to save original file as attachment: {}", e);
    }
    println!("Invoice imported with id={}", id);
    Ok(())
}
