use anyhow::Result;
use bitcoin::{blockdata::script, ScriptBuf};

pub fn bip34_block_height(coinbase_prefix: &[u8]) -> Result<u64> {
    let script = ScriptBuf::from_bytes(coinbase_prefix.to_owned());
    let mut instructions = script.instructions_minimal();
    let push = instructions
        .next()
        .ok_or_else(|| anyhow::anyhow!("No instructions in script for BIP34 block height"))??;
    match (
        push.script_num(),
        push.push_bytes()
            .map(|b| script::read_scriptint(b.as_bytes())),
    ) {
        (Some(num), Some(Ok(_)) | None) => Ok(num
            .try_into()
            .map_err(|e| anyhow::anyhow!("Negative Height: {}", e))?),
        (_, Some(Err(err))) => Err(anyhow::anyhow!("Invalid BIP34 coinbase prefix: {}", err)),
        (None, _) => Err(anyhow::anyhow!(
            "Invalid BIP34 coinbase prefix: no push bytes found"
        )),
    }
}
