use anyhow::{anyhow, Result};
use colored::Colorize;
use lightning_invoice::Bolt11Invoice;
use rusqlite::{Connection, OptionalExtension, Row};
use std::env;

const DATABASE_PATH: &str = "./graph.db3";

fn main() -> Result<()> {
    let graph_database = GraphDatabase::open(DATABASE_PATH)?;

    let invoice = env::args()
        .nth(1)
        .ok_or(anyhow!("BOLT-11 invoice is required"))?
        .parse::<Bolt11Invoice>()?;

    let amount = format_msat(invoice.amount_milli_satoshis());
    let description = invoice.description().to_string();
    let pubkey = invoice
        .payee_pub_key()
        .copied()
        .unwrap_or_else(|| invoice.recover_payee_pub_key())
        .to_string();
    let alias = graph_database.query_alias(&pubkey)?;
    let alias = format_alias(&alias);

    println!("        Amount: {amount}");
    println!("   Description: {description}");
    println!("         Payee: {pubkey} {alias}");

    for hint in invoice.route_hints() {
        println!("   Routing hint:");
        for hop in hint.0 {
            let pubkey = hop.src_node_id.to_string();
            let alias = graph_database.query_alias(&pubkey)?;
            let alias = format_alias(&alias);
            println!("              Hop: {pubkey} {alias}");
        }
    }

    Ok(())
}

fn format_alias(alias: &Option<String>) -> String {
    match alias {
        Some(alias) => format!("public node with alias {}", alias.bold()),
        None => "private node".to_string(),
    }
}

struct GraphDatabase {
    connection: Connection,
}

impl GraphDatabase {
    fn open(database_path: &str) -> Result<Self> {
        let connection = Connection::open(database_path)?;
        Ok(Self { connection })
    }

    fn query_alias(&self, pubkey: &str) -> Result<Option<String>> {
        Ok(self
            .connection
            .query_row(
                "SELECT alias FROM nodes WHERE pubkey = ?1 LIMIT 1",
                [pubkey],
                |row: &Row| row.get::<usize, String>(0),
            )
            .optional()?)
    }
}

fn format_msat(msat: Option<u64>) -> String {
    match msat {
        None => "no amount".to_string(),
        Some(msat) if msat % 1000 == 0 => format!("{} sats", msat / 1000),
        Some(msat) => {
            let sat = msat / 1000;
            let msat = msat % 1000;
            format!("{sat}.{msat:03} sats")
        }
    }
}
