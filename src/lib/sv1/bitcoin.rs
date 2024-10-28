use bitcoin::address::{NetworkChecked, NetworkUnchecked};
use bitcoin::{Address, Network};
use bitcoincore_rpc::RpcApi;
use bitcoincore_rpc_json::{GetBlockTemplateModes, GetBlockTemplateResult, GetBlockTemplateRules};
use std::str::FromStr;
use sv1_api::utils::{HexBytes, HexU32Be, MerkleNode, PrevHash};

#[derive(Clone)]
pub struct Template {
    pub prevhash: PrevHash<'static>,
    pub coinbase_prefix: HexBytes,
    pub coinbase_suffix: HexBytes,
    pub merkle_branches: Vec<MerkleNode<'static>>,
    pub version: HexU32Be,
    pub bits: HexU32Be,
    pub time: HexU32Be,
}

impl Template {
    pub fn new(
        bitcoin_network: String,
        solo_miner_signature: String,
        solo_miner_address: String,
    ) -> anyhow::Result<Self> {
        let network = match bitcoin_network.as_str() {
            "mainnet" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "signet" => Network::Signet,
            "regtest" => Network::Regtest,
            _ => panic!("invalid network"),
        };
        let address: Address<NetworkUnchecked> = Address::from_str(&solo_miner_address)?;
        let address: Address<NetworkChecked> = address.require_network(network)?;
        let script_pubkey = address.script_pubkey();

        let mut coinbase_prefix = Vec::new();
        coinbase_prefix.extend_from_slice(solo_miner_signature.as_bytes());

        let mut coinbase_suffix = Vec::new();
        coinbase_suffix.extend_from_slice(&script_pubkey.to_bytes());

        Ok(Self {
            prevhash: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
                .try_into()
                .expect("should always work"),
            coinbase_prefix: coinbase_prefix.try_into().expect("should always work"),
            coinbase_suffix: coinbase_suffix.try_into().expect("should always work"),
            merkle_branches: vec![],
            version: 0.try_into().expect("should always work"),
            bits: 0.try_into().expect("should always work"),
            time: 0.try_into().expect("should always work"),
        })
    }

    pub fn update(&mut self, gbt_result: GetBlockTemplateResult) {
        let prevhash = gbt_result.previous_block_hash.to_raw_hash();
        let prevhash_bytes: &[u8; 32] = prevhash.as_ref();
        let prevhash_string = hex::encode(invert_endianness(prevhash_bytes));
        let prevhash: PrevHash<'static> = prevhash_string
            .as_str()
            .try_into()
            .expect("should always work");

        let version = HexU32Be(gbt_result.version);
        let bits: HexU32Be = hex::encode(gbt_result.bits)
            .as_str()
            .try_into()
            .expect("should always work");
        let time = HexU32Be(gbt_result.min_time as u32);

        self.prevhash = prevhash;
        self.version = version;
        self.bits = bits;
        self.time = time;
    }
}

fn invert_endianness(hash: &[u8; 32]) -> [u8; 32] {
    let mut inverted = [0u8; 32];
    for i in 0..8 {
        let start = i * 4;
        inverted[start] = hash[start + 3];
        inverted[start + 1] = hash[start + 2];
        inverted[start + 2] = hash[start + 1];
        inverted[start + 3] = hash[start];
    }
    inverted
}

const BLOCK_TEMPLATE_RULES: [GetBlockTemplateRules; 4] = [
    GetBlockTemplateRules::SegWit,
    GetBlockTemplateRules::Signet,
    GetBlockTemplateRules::Csv,
    GetBlockTemplateRules::Taproot,
];

const RPC_BACKOFF_BASE: u64 = 2;
const MAX_RPC_FAILURES: u32 = 20;

pub async fn gbt(rpc: &bitcoincore_rpc::Client) -> GetBlockTemplateResult {
    let mut rpc_failure_counter = 0;
    let mut rpc_failure_backoff;

    loop {
        match rpc.get_block_template(GetBlockTemplateModes::Template, &BLOCK_TEMPLATE_RULES, &[]) {
            Ok(get_block_template_result) => {
                return get_block_template_result;
            }
            Err(e) => {
                rpc_failure_counter += 1;
                if rpc_failure_counter > MAX_RPC_FAILURES {
                    tracing::error!(
                        "Exceeded the maximum number of failed `getblocktemplate` RPC \
                    attempts. Halting."
                    );
                    std::process::exit(1);
                }
                rpc_failure_backoff =
                    u64::checked_pow(RPC_BACKOFF_BASE, rpc_failure_counter.clone())
                        .expect("MAX_RPC_FAILURES doesn't allow overflow; qed");

                // sleep until it's time to try again
                tracing::error!("Error on getblocktemplate RPC: {}", e);
                tracing::error!(
                    "Exponential Backoff: getblocktemplate RPC failed {} times, waiting {} \
                    seconds before attempting getblocktemplate RPC again.",
                    rpc_failure_counter,
                    rpc_failure_backoff
                );
                std::thread::sleep(std::time::Duration::from_secs(rpc_failure_backoff));
            }
        }
    }
}
