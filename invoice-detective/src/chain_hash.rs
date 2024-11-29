use std::fmt::Display;

use bitcoin::hex::DisplayHex;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChainHash {
    Bitcoin,
    Regtest,
    Signet,
    Testnet,
    Unknown(bitcoin::constants::ChainHash),
}

impl From<bitcoin::constants::ChainHash> for ChainHash {
    fn from(chain: bitcoin::constants::ChainHash) -> Self {
        match chain {
            bitcoin::constants::ChainHash::BITCOIN => Self::Bitcoin,
            bitcoin::constants::ChainHash::REGTEST => Self::Regtest,
            bitcoin::constants::ChainHash::SIGNET => Self::Signet,
            bitcoin::constants::ChainHash::TESTNET => Self::Testnet,
            chain => Self::Unknown(chain),
        }
    }
}

impl Display for ChainHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ChainHash::Unknown(chain_hash) => write!(f, "{}", chain_hash.as_bytes().as_hex()),
            network => write!(f, "{:?}", network),
        }
    }
}
