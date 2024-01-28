#[derive(Debug)]
pub struct Node {
    pub pubkey: String,
    pub alias: Option<String>,
    pub is_announced: bool,
}
