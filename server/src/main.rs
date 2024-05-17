use invoice_detective::{InvoiceDetective, RecipientNode, ServiceKind};
use rocket::{get, launch, routes};
use rocket_dyn_templates::{context, Template};
use thousands::Separable;

#[get("/")]
fn index() -> Template {
    Template::render("index", context![])
}

#[get("/<invoice>")]
fn invoice(invoice: &str) -> Result<Template, String> {
    let invoice_detective = InvoiceDetective::new()
        .map_err(|e| format!("Failed to initialize InvoiceDetective: {e}"))?;
    let findings = match invoice_detective.investigate(invoice) {
        Ok(findings) => findings,
        Err(e) => {
            return Ok(Template::render(
                "invoice",
                context! { invoice, error: e.to_string() },
            ))
        }
    };

    let recipient = findings.recipient;
    let payee = findings.payee;
    let route_hints = findings.route_hints;

    let (custody, service, name, id) = match recipient {
        RecipientNode::Custodial { custodian } => (
            "Custodial",
            format_service_kind(&custodian.service),
            custodian.name,
            String::new(),
        ),
        RecipientNode::NonCustodial { id, lsp } => (
            "Non-custodial",
            format_service_kind(&lsp.service),
            lsp.name,
            id,
        ),
        RecipientNode::NonCustodialWrapped { lsp } => (
            "Non-custodial",
            format_service_kind(&lsp.service),
            lsp.name,
            String::new(),
        ),
        RecipientNode::Unknown => ("Unknown", "", String::new(), String::new()),
    };

    let amount = format_msat(findings.details.amount_msat);
    let description = findings.details.description;
    let network = findings.details.network;

    let mempool_space_base_url = "https://mempool.space/lightning/node";
    Ok(Template::render(
        "invoice",
        context! { amount, network, description, invoice, mempool_space_base_url, route_hints, payee, custody, service, name, id },
    ))
}

fn format_msat(msat: Option<u64>) -> String {
    match msat {
        None => String::new(),
        Some(1000) => "1 sat".to_string(),
        Some(msat) if msat % 1000 == 0 => format!("{} sats", (msat / 1000).separate_with_commas()),
        Some(msat) => {
            let sat = msat / 1000;
            let sat = sat.separate_with_commas();
            let msat = msat % 1000;
            format!("{sat}.{msat:03} sats")
        }
    }
}

#[launch]
fn rocket() -> _ {
    // TODO: Customize template directory.
    rocket::build()
        .mount("/", routes![index, invoice])
        .attach(Template::fairing())
}

fn format_service_kind(service: &ServiceKind) -> &'static str {
    match service {
        ServiceKind::BusinessWallet => "Payment processor",
        ServiceKind::ConsumerWallet => "Consumer wallet",
        ServiceKind::Exchange => "Exchange",
        ServiceKind::Lsp => "LSP",
    }
}
