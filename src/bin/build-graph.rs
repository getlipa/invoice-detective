use anyhow::{anyhow, bail, Context, Result};
use rusqlite::Connection;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{self, Read},
};

const DATABASE_PATH: &str = "./graph.db3";

#[derive(Debug)]
struct Node {
    pubkey: String,
    alias: String,
    capacity: u64,
}

type NodesMap = HashMap<String, Node>;

pub fn main() -> Result<()> {
    let graph_file = env::args()
        .nth(1)
        .ok_or(anyhow!("JSON of Lightning Network graph is required"))?;
    println!("Reading {graph_file} ...");

    let graph_file = File::open(graph_file)?;
    let mut buf_reader = io::BufReader::new(graph_file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;

    println!("Parsing ...");
    let graph = json::parse(&contents)?;
    let graph = as_object(&graph)?;

    let nodes = graph
        .get("nodes")
        .ok_or(anyhow!("Invalid JSON: missing nodes"))?;
    let nodes = as_array(nodes)?;

    println!("Processing nodes ...");
    let mut nodes = process_nodes(nodes)?;

    let edges = graph
        .get("edges")
        .ok_or(anyhow!("Invalid JSON: missing edges"))?;
    let edges = as_array(edges)?;
    println!("Processing edges ...");
    process_edges(edges, &mut nodes)?;

    println!("Clustering ...");

    println!("Dumping to database {DATABASE_PATH} ...");
    dump(&nodes)?;

    println!("Done");
    Ok(())
}

const CREATE_DB: &str = "
DROP TABLE IF EXISTS nodes;
CREATE TABLE nodes (
    id       INTEGER NOT NULL PRIMARY KEY,
    pubkey   TEXT NOT NULL,
    alias    TEST NOT NULL,
    capacity INTEGER NOT NULL
);";
const INSERT_NODE: &str = "
INSERT INTO nodes(pubkey, alias, capacity)
VALUES (?1, ?2, ?3)";

fn dump(nodes: &NodesMap) -> Result<()> {
    let mut connection = Connection::open(DATABASE_PATH)?;
    connection.execute_batch(CREATE_DB)?;

    let transaction = connection.transaction()?;
    {
        let mut statement = transaction.prepare(INSERT_NODE)?;
        for node in nodes.values() {
            statement.execute([&node.pubkey, &node.alias, &node.capacity.to_string()])?;
        }
    }
    transaction.commit()?;

    Ok(())
}

fn process_nodes(nodes: &json::Array) -> Result<NodesMap> {
    let mut nodes_map = HashMap::with_capacity(nodes.len());
    for node in nodes {
        let node = as_object(node)?;
        let pubkey = get_str(node, "pub_key")?;
        let alias = get_str(node, "alias")?;

        let node = Node {
            pubkey: pubkey.to_string(),
            alias: alias.to_string(),
            capacity: 0,
        };

        nodes_map.insert(pubkey.to_string(), node);
    }
    Ok(nodes_map)
}

fn process_edges(edges: &json::Array, nodes: &mut NodesMap) -> Result<()> {
    for edge in edges {
        let edge = as_object(edge)?;
        let node1 = get_str(edge, "node1_pub")?;
        let node2 = get_str(edge, "node2_pub")?;
        let capacity = get_str(edge, "capacity")?;
        let capacity: u64 = capacity
            .parse()
            .context("channel capacity is not a number")?;

        if let Some(node) = nodes.get_mut(node1) {
            node.capacity += capacity;
        }
        if let Some(node) = nodes.get_mut(node2) {
            node.capacity += capacity;
        }
    }
    Ok(())
}

fn as_array(json: &json::JsonValue) -> Result<&json::Array> {
    match json {
        json::JsonValue::Array(array) => Ok(array),
        _ => bail!("Not an array"),
    }
}

fn as_object(json: &json::JsonValue) -> Result<&json::object::Object> {
    match json {
        json::JsonValue::Object(object) => Ok(object),
        _ => bail!("Not an object"),
    }
}

fn get_str<'a>(json: &'a json::object::Object, key: &'static str) -> Result<&'a str> {
    let value = json.get(key).ok_or(anyhow!(format!("missing {key}")))?;
    value
        .as_str()
        .ok_or(anyhow!(format!("{key} is not a string")))
}
