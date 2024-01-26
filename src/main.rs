mod recipient;

use crate::recipient::{RecipientDecoder, RecipientNode, ServiceKind};

use anyhow::{anyhow, Result};
use colored::Colorize;
use lightning_invoice::{Bolt11Invoice, RouteHint};
use rusqlite::{Connection, OptionalExtension, Row};
use std::env;
use thousands::Separable;

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
    let node_name = graph_database.query_name(pubkey.clone())?;
    let route_hints = process_route_hints(&invoice.route_hints(), &graph_database)?;

    println!("🔎 {}", " Investigative findings ".reversed());
    let recipient_decoder = RecipientDecoder::new();
    let recipient = recipient_decoder.decode(&pubkey, &route_hints);
    let recipient = format_recipient_node(&recipient);
    println!("   Recipient: {recipient}");

    println!();
    println!("🧾 {}", " Evidences ".reversed());
    println!(
        "   Pay {} to {}",
        amount.bold(),
        format_node_name(&node_name)
    );
    for hint in route_hints {
        let hint = hint
            .iter()
            .map(format_node_name)
            .collect::<Vec<_>>()
            .join(" → ");
        println!("     via {hint}");
    }
    if !description.is_empty() {
        println!("   Description: {}", description.italic());
    }
    Ok(())
}

fn format_service_kind(service: &ServiceKind) -> &str {
    match service {
        ServiceKind::BusinessWallet => "Payment processor",
        ServiceKind::ConsumerWallet => "Consumer wallet",
        ServiceKind::Exchange => "Exchange",
        ServiceKind::Lsp => "LSP",
    }
}

fn format_recipient_node(node: &RecipientNode) -> String {
    match node {
        RecipientNode::Custodial { custodian } => format!(
            "Custodial {} {}",
            format_service_kind(&custodian.service),
            custodian.name.bold()
        ),
        RecipientNode::NonCustodial { id, lsp } => format!(
            "Non-custodial {} {} with id:{}",
            format_service_kind(&lsp.service),
            lsp.name.bold(),
            id.bold()
        ),
        RecipientNode::NonCustodialWrapped { lsp } => {
            format!(
                "Non-custodial {} {}",
                format_service_kind(&lsp.service),
                lsp.name.bold()
            )
        }
        RecipientNode::Unknown => "Unknown".to_string(),
    }
}

#[derive(Clone)]
enum NodeName {
    PublicNodeAlias(String, String),
    PublicNodePubkey(String),
    PrivateNodePubkey(String),
}

impl NodeName {
    fn get_alias(&self) -> &String {
        match self {
            NodeName::PublicNodeAlias(_alias, pubkey) => pubkey,
            NodeName::PublicNodePubkey(pubkey) => pubkey,
            NodeName::PrivateNodePubkey(pubkey) => pubkey,
        }
    }
}

fn format_node_name(node_name: &NodeName) -> String {
    match node_name {
        NodeName::PublicNodeAlias(alias, _) => format!("public node alias:{}", alias.bold()),
        NodeName::PublicNodePubkey(pubkey) => format!("public node id:{}", pubkey.bold()),
        NodeName::PrivateNodePubkey(pubkey) => format!("private node id:{}", pubkey.bold()),
    }
}

fn process_route_hints(
    route_hints: &Vec<RouteHint>,
    graph_database: &GraphDatabase,
) -> Result<Vec<Vec<NodeName>>> {
    let mut result = Vec::new();
    for hint in route_hints {
        let mut x = Vec::new();
        for hop in &hint.0 {
            let node_name = graph_database.query_name(hop.src_node_id.to_string())?;
            x.push(node_name);
        }
        result.push(x);
    }
    Ok(result)
}

struct GraphDatabase {
    connection: Connection,
}

impl GraphDatabase {
    fn open(database_path: &str) -> Result<Self> {
        let connection = Connection::open(database_path)?;
        Ok(Self { connection })
    }

    fn query_name(&self, pubkey: String) -> Result<NodeName> {
        let node_name = match self.query_alias(&pubkey)? {
            Some(alias) if !alias.is_empty() => NodeName::PublicNodeAlias(alias, pubkey),
            Some(pubkey) => NodeName::PublicNodePubkey(pubkey),
            None => NodeName::PrivateNodePubkey(pubkey),
        };
        Ok(node_name)
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
        None => "any amount".to_string(),
        Some(msat) if msat % 1000 == 0 => {
            let sat = msat / 1000;
            let sat = sat.separate_with_commas();
            format!("{sat} sats")
        }
        Some(msat) => {
            let sat = msat / 1000;
            let sat = sat.separate_with_commas();
            let msat = msat % 1000;
            format!("{sat}.{msat:03} sats")
        }
    }
}
