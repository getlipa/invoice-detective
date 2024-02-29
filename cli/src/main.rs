use anyhow::{anyhow, Result};
use colored::Colorize;
use invoice_detective::decoder::{decode, resolve};
use invoice_detective::{InvoiceDetective, Node, RecipientNode, ServiceKind};
use std::env;
use thousands::Separable;

#[tokio::main]
async fn main() -> Result<()> {
    let input = env::args().nth(1).ok_or(anyhow!("Input is required"))?;
    let decoded_data = decode(&input)?;
    let invoice = resolve(decoded_data).await?;

    let invoice_detective = InvoiceDetective::new()?;
    let findings = invoice_detective.investigate(&invoice)?;

    println!("🔎 {}", " Investigative findings ".reversed());
    let recipient = format_recipient_node(&findings.recipient);
    println!("   Recipient: {recipient}");

    println!();
    println!("🗃️  {}", " Evidences ".reversed());
    println!("   Pay to {}", format_node_name(&findings.payee));
    for hint in findings.route_hints {
        let hint = hint
            .iter()
            .map(format_node_name)
            .collect::<Vec<_>>()
            .join(" → ");
        println!("     via {hint}");
    }

    let details = findings.details;
    let amount = format_msat(details.amount_msat);
    println!();
    println!("📋 {}", " Details ".reversed());
    println!("    Network: {}", details.network);
    println!("     Amount: {amount}");
    println!("Desctiption: {}", details.description.italic());

    Ok(())
}

fn format_msat(msat: Option<u64>) -> String {
    match msat {
        None => "empty".to_string(),
        Some(msat) if msat % 1000 == 0 => format!("{} sats", (msat / 1000).separate_with_commas()),
        Some(msat) => {
            let sat = msat / 1000;
            let sat = sat.separate_with_commas();
            let msat = msat % 1000;
            format!("{sat}.{msat:03} sats")
        }
    }
}

fn format_node_name(node: &Node) -> String {
    let visibility = match node.is_announced {
        true => "public",
        false => "private",
    };
    match &node.alias {
        Some(alias) => format!("{visibility} node alias:{}", alias.bold()),
        None => format!("{visibility} node id:{}", node.pubkey.bold()),
    }
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
