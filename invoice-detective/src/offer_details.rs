use crate::chain_hash::ChainHash;
use bitcoin::hex::DisplayHex;
use chrono::{DateTime, Utc};
use core::str;
use lightning::offers::offer::{Offer, Quantity};
use lightning::util::scid_utils::{block_from_scid, tx_index_from_scid, vout_from_scid};
use std::fmt::Display;

#[derive(Debug)]
pub struct Amount {
    amount: u64,
    iso4217_code: String,
    exponent: u16,
}

impl From<lightning::offers::offer::Amount> for Amount {
    fn from(amount: lightning::offers::offer::Amount) -> Self {
        match amount {
            lightning::offers::offer::Amount::Currency {
                iso4217_code,
                amount,
            } => {
                if let Ok(code) = str::from_utf8(&iso4217_code) {
                    if let Some(currency) = iso_currency::Currency::from_code(code) {
                        return Self {
                            amount,
                            iso4217_code: currency.code().to_string(),
                            exponent: currency.exponent().unwrap_or(0),
                        };
                    }
                }
                Self {
                    amount,
                    iso4217_code: iso4217_code.as_hex().to_string(),
                    exponent: 0,
                }
            }
            lightning::offers::offer::Amount::Bitcoin { amount_msats } => Self {
                amount: amount_msats,
                iso4217_code: "SAT".to_string(),
                exponent: 3,
            },
        }
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut amount = self.amount.to_string();
        if self.exponent == 0 {
            write!(f, "{amount}")?;
        } else if amount.len() <= self.exponent as usize {
            let leading_zeros = self.exponent as usize - amount.len();
            let leading_zeros: String = "0".repeat(leading_zeros);
            write!(f, "0.{leading_zeros}{amount}")?;
        } else {
            let integer_len = amount.len() - self.exponent as usize;
            let fraction = amount.split_off(integer_len);
            if fraction.chars().any(|c| c != '0') {
                write!(f, "{amount}.{fraction}")?;
            } else {
                write!(f, "{amount}")?;
            }
        }
        write!(f, " {}", self.iso4217_code)
    }
}

#[derive(Debug)]
pub struct OfferDetails {
    pub id: String,
    pub chains: Vec<String>,
    pub amount: Option<Amount>,
    pub supported_quantity: String,
    pub description: Option<String>,
    pub issuer: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<String>,
    pub signing_pubkey: Option<String>,
    pub paths: Vec<BlindedPath>,
}

#[derive(Debug)]
pub struct BlindedHop {
    pub node_id: String,
    pub encrypted_payload: String,
}

impl From<&lightning::blinded_path::BlindedHop> for BlindedHop {
    fn from(hop: &lightning::blinded_path::BlindedHop) -> Self {
        Self {
            node_id: hop.blinded_node_id.to_string(),
            encrypted_payload: hop.encrypted_payload.as_hex().to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ShortChannelId {
    pub block_height: u32,
    pub transaction_index: u32,
    pub output_index: u16,
}

impl From<u64> for ShortChannelId {
    fn from(scid: u64) -> Self {
        Self {
            block_height: block_from_scid(scid),
            transaction_index: tx_index_from_scid(scid),
            output_index: vout_from_scid(scid),
        }
    }
}

impl Display for ShortChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}x{}x{}",
            self.block_height, self.transaction_index, self.output_index
        )
    }
}

#[derive(Debug)]
pub enum IntroductionNode {
    NodeId(String),
    LeftEnd(ShortChannelId),
    RightEnd(ShortChannelId),
}

impl From<&lightning::blinded_path::IntroductionNode> for IntroductionNode {
    fn from(node: &lightning::blinded_path::IntroductionNode) -> Self {
        use lightning::blinded_path::Direction;
        use lightning::blinded_path::IntroductionNode as LdkNode;
        match node {
            LdkNode::NodeId(pubkey) => Self::NodeId(pubkey.to_string()),
            LdkNode::DirectedShortChannelId(Direction::NodeOne, channel) => {
                Self::LeftEnd(ShortChannelId::from(*channel))
            }
            LdkNode::DirectedShortChannelId(Direction::NodeTwo, channel) => {
                Self::RightEnd(ShortChannelId::from(*channel))
            }
        }
    }
}

#[derive(Debug)]
pub struct BlindedPath {
    pub introduction_node: IntroductionNode,
    pub blinding_point: String,
    pub hops: Vec<BlindedHop>,
}

impl From<&lightning::blinded_path::message::BlindedMessagePath> for BlindedPath {
    fn from(path: &lightning::blinded_path::message::BlindedMessagePath) -> Self {
        Self {
            introduction_node: path.introduction_node().into(),
            blinding_point: path.blinding_point().to_string(),
            hops: path.blinded_hops().iter().map(BlindedHop::from).collect(),
        }
    }
}

impl From<Offer> for OfferDetails {
    fn from(offer: Offer) -> Self {
        let mut chains = offer
            .chains()
            .into_iter()
            .map(ChainHash::from)
            .collect::<Vec<_>>();
        chains.sort();
        let chains = chains.into_iter().map(|c| c.to_string()).collect();

        let amount = offer.amount().map(Amount::from);
        let supported_quantity = match offer.supported_quantity() {
            Quantity::Unbounded => "Any".to_string(),
            Quantity::One => "One".to_string(),
            Quantity::Bounded(n) => format!("No more than {n}"),
        };
        let description = offer.description().map(|s| s.to_string());
        let issuer = offer.issuer().map(|s| s.to_string());
        let expires_at = offer
            .absolute_expiry()
            .map(|d| DateTime::from_timestamp(d.as_secs() as i64, 0).unwrap());
        let metadata = offer.metadata().map(|s| s.as_hex().to_string());

        let signing_pubkey = offer.signing_pubkey().map(|k| k.to_string());
        let paths = offer.paths().iter().map(BlindedPath::from).collect();

        Self {
            id: offer.id().0.as_hex().to_string(),
            chains,
            amount,
            supported_quantity,
            description,
            issuer,
            expires_at,
            metadata,
            signing_pubkey,
            paths,
        }
    }
}
