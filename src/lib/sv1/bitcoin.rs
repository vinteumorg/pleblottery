use sv1_api::utils::{HexBytes, HexU32Be, MerkleNode, PrevHash};

#[derive(Clone)]
pub struct SoloHeaderFields {
    pub prevhash: PrevHash<'static>,
    pub coinbase_prefix: HexBytes,
    pub coinbase_suffix: HexBytes,
    pub merkle_branches: Vec<MerkleNode<'static>>,
    pub version: HexU32Be,
    pub bits: HexU32Be,
    pub time: HexU32Be,
}

impl SoloHeaderFields {
    pub fn new() -> Self {
        Self {
            prevhash: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
                .try_into()
                .expect("should always work"),
            coinbase_prefix: "".try_into().expect("should always work"),
            coinbase_suffix: "".try_into().expect("should always work"),
            merkle_branches: vec![],
            version: 0.try_into().expect("should always work"),
            bits: 0.try_into().expect("should always work"),
            time: 0.try_into().expect("should always work"),
        }
    }
}
