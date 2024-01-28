use crate::node::Node;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, Row};

pub struct GraphDatabase {
    connection: Connection,
}

impl GraphDatabase {
    pub fn open(database_path: &str) -> Result<Self> {
        let connection = Connection::open(database_path)?;
        Ok(Self { connection })
    }

    pub fn query(&self, pubkey: String) -> Result<Node> {
        let node = match self.query_alias(&pubkey)? {
            Some(alias) if !alias.is_empty() => Node {
                pubkey,
                alias: Some(alias),
                is_announced: true,
            },
            Some(_alias) => Node {
                pubkey,
                alias: None,
                is_announced: true,
            },
            None => Node {
                pubkey,
                alias: None,
                is_announced: false,
            },
        };
        Ok(node)
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
