mod chain_hash;
pub mod decoder;
mod graph_database;
mod node;
pub mod offer_details;
mod recipient;

use crate::graph_database::GraphDatabase;
pub use crate::node::Node;
use crate::recipient::RecipientDecoder;
pub use crate::recipient::{RecipientNode, ServiceKind};
use anyhow::{anyhow, Result};
use bitcoin::secp256k1::PublicKey;
use lightning::blinded_path::message::BlindedMessagePath;
use lightning::blinded_path::IntroductionNode;
use lightning::offers::offer::Offer;
use lightning_invoice::{Bolt11Invoice, Currency, RouteHint};

#[derive(Debug, Default)]
pub struct InvoiceDetails {
    pub network: &'static str,
    pub description: String,
    pub amount_msat: Option<u64>,
    // pub date: u64,
    // pub expires_at: u64,
}

#[derive(Debug)]
pub struct InvestigativeFindings {
    pub recipient: RecipientNode,
    pub payee: Node,
    pub route_hints: Vec<Vec<Node>>,
    pub details: InvoiceDetails,
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
        let invoice = invoice.trim().parse::<Bolt11Invoice>()?;
        self.investigate_bolt11(invoice)
    }

    pub fn investigate_bolt11(&self, invoice: Bolt11Invoice) -> Result<InvestigativeFindings> {
        let description = invoice.description().to_string();
        let pubkey = invoice
            .payee_pub_key()
            .copied()
            .unwrap_or_else(|| invoice.recover_payee_pub_key())
            .to_string();
        let payee = self.graph_database.query(pubkey.clone())?;
        let route_hints = self.process_route_hints(&invoice.route_hints())?;
        let recipient = self.recipient_decoder.decode(&pubkey, &route_hints);

        let network = match invoice.currency() {
            Currency::Bitcoin => "Mainnet",
            Currency::BitcoinTestnet => "Testnet",
            Currency::Regtest => "Regtest",
            Currency::Simnet => "Simnet",
            Currency::Signet => "Signet",
        };

        let details = InvoiceDetails {
            network,
            description,
            amount_msat: invoice.amount_milli_satoshis(),
        };

        Ok(InvestigativeFindings {
            recipient,
            payee,
            route_hints,
            details,
        })
    }

    pub fn investigate_bolt12(&self, offer: Offer) -> Result<InvestigativeFindings> {
        let introduction_node = offer
            .paths()
            .first()
            .map(BlindedMessagePath::introduction_node);
        let destination = match introduction_node {
            Some(IntroductionNode::NodeId(introduction_node_id)) => Destination::Blinded {
                introduction_node_id: *introduction_node_id,
            },
            Some(IntroductionNode::DirectedShortChannelId(_direction, _channel_id)) => {
                unimplemented!();
            }
            None => Destination::Node(
                offer
                    .signing_pubkey()
                    .ok_or(anyhow!("Blinded path and signing key are empty"))?,
            ),
        };
        let pubkey = destination.pubkey().to_string();
        let payee = self.graph_database.query(pubkey.clone())?;
        let recipient = self.recipient_decoder.decode(&pubkey, &Vec::new());

        let details = InvoiceDetails::default();
        Ok(InvestigativeFindings {
            recipient,
            payee,
            route_hints: Vec::new(),
            details,
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

#[derive(Debug)]
enum Destination {
    Node(PublicKey),
    Blinded { introduction_node_id: PublicKey },
}

impl Destination {
    fn pubkey(&self) -> &PublicKey {
        match self {
            Destination::Node(key) => key,
            Destination::Blinded {
                introduction_node_id,
            } => introduction_node_id,
        }
    }
}
