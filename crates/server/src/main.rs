use rocket::{get, launch, routes};
// use rocket_dyn_templates::serde::Serialize;
use invoice_detective::{InvoiceDetective, Node, RecipientNode, ServiceKind};
use rocket_dyn_templates::{context, Template};

#[get("/<invoice>")]
fn index(invoice: &str) -> Template {
    let invoice_detective = InvoiceDetective::new().unwrap();
    let findings = invoice_detective.investigate(invoice).unwrap();

    let recipient = format_recipient_node(&findings.recipient);
    let payee = format_node_name(&findings.payee);
    let route_hints = format_route_hints(&findings.route_hints);

    let mempool_space_base_url = "https://mempool.space/lightning/node";
    Template::render(
        "index",
        context! { invoice, recipient, mempool_space_base_url, payee, route_hints },
    )
}

#[launch]
fn rocket() -> _ {
    // TODO: Customize template directory.
    rocket::build()
        .mount("/", routes![index])
        .attach(Template::fairing())
}

fn format_route_hints(route_hints: &Vec<Vec<Node>>) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for hint in route_hints {
        let x = hint
            .iter()
            .map(|n| n.alias.clone().unwrap_or(n.pubkey.clone()))
            .collect();
        result.push(x);
    }
    result
}

fn format_node_name(node: &Node) -> String {
    let visibility = match node.is_announced {
        true => "public",
        false => "private",
    };
    match &node.alias {
        Some(alias) => format!("{visibility} node alias:{}", alias),
        None => format!("{visibility} node id:{}", node.pubkey),
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
            custodian.name
        ),
        RecipientNode::NonCustodial { id, lsp } => format!(
            "Non-custodial {} {} with id:{}",
            format_service_kind(&lsp.service),
            lsp.name,
            id
        ),
        RecipientNode::NonCustodialWrapped { lsp } => {
            format!(
                "Non-custodial {} {}",
                format_service_kind(&lsp.service),
                lsp.name
            )
        }
        RecipientNode::Unknown => "Unknown".to_string(),
    }
}
