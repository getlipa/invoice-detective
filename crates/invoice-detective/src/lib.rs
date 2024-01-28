mod graph_database;
mod node;
mod recipient;

use crate::graph_database::GraphDatabase;
pub use crate::node::Node;
use crate::recipient::RecipientDecoder;
pub use crate::recipient::{RecipientNode, ServiceKind};

use anyhow::Result;
use lightning_invoice::{Bolt11Invoice, RouteHint};

#[derive(Debug)]
pub struct InvestigativeFindings {
    pub recipient: RecipientNode,
    pub payee: Node,
    pub route_hints: Vec<Vec<Node>>,
}

pub struct InvoiceDetective {
    graph_database: GraphDatabase,
    recipient_decoder: RecipientDecoder,
}

impl InvoiceDetective {
    pub fn new() -> Result<Self> {
        const DATABASE_PATH: &str = "./graph.db3";
        let graph_database = GraphDatabase::open(DATABASE_PATH)?;
        let recipient_decoder = RecipientDecoder::new();
        Ok(Self {
            graph_database,
            recipient_decoder,
        })
    }

    pub fn investigate(&self, invoice: &str) -> Result<InvestigativeFindings> {
        let invoice = invoice.parse::<Bolt11Invoice>()?;
        let _description = invoice.description().to_string();
        let pubkey = invoice
            .payee_pub_key()
            .copied()
            .unwrap_or_else(|| invoice.recover_payee_pub_key())
            .to_string();
        let payee = self.graph_database.query(pubkey.clone())?;
        let route_hints = self.process_route_hints(&invoice.route_hints())?;
        let recipient = self.recipient_decoder.decode(&pubkey, &route_hints);

        Ok(InvestigativeFindings {
            recipient,
            payee,
            route_hints,
        })
    }

    fn process_route_hints(&self, route_hints: &Vec<RouteHint>) -> Result<Vec<Vec<Node>>> {
        let mut result = Vec::new();
        for hint in route_hints {
            let mut x = Vec::new();
            for hop in &hint.0 {
                let node = self.graph_database.query(hop.src_node_id.to_string())?;
                x.push(node);
            }
            result.push(x);
        }
        Ok(result)
    }
}
