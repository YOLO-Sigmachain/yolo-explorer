//! Observed mainnet EIP-27 emission rewards. Never infer net miner income from height alone.
use crate::budget::Budget;
use crate::{blocking, ApiError, AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use xp_store::{
    read::{Dir, ExpansionRow},
    rows::BoxRow,
    Reader,
};
use xp_types::Hash32;

const EMISSION_NFT: &str = "20fa2bf23962cdf51b07722d6237c0c7b8a44f78856c0f7ec308dc1ef1a92a51";
const REEMISSION_TOKEN: &str = "d9a2cc8a09abfaed87afacfbb7daee79a6b26f10c6613fc13d3f3953e5521d1a";

#[derive(Serialize)]
pub struct RewardBreakdown {
    pub block_id: String,
    pub height: u32,
    pub basis: &'static str,
    pub gross_reward: Option<String>,
    pub reemission_obligation: Option<String>,
    pub miner_subsidy: Option<String>,
    /// Split-emission chains only: the treasury's share of this block's emission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub treasury_reward: Option<String>,
    /// Split-emission chains only: the liquidity fund's share of this block's emission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lp_reward: Option<String>,
    pub transaction_fees: String,
    pub reward_box_id: Option<String>,
    pub transaction_id: Option<String>,
    pub note: &'static str,
}

fn read_box(rd: &Reader, budget: &mut Budget, id: &Hash32) -> Result<BoxRow, ApiError> {
    budget.admit_row(rd, ExpansionRow::Box, id)?;
    rd.box_by_id_checked(id, || {
        budget
            .check()
            .map_err(|_| xp_store::StoreError::ReadLimit("expansion_deadline"))
    })?
    .ok_or_else(|| ApiError::Integrity("missing reward output".into()))
}

// Mainnet's 720-block delayed miner reward contract, with the header's compressed key.
// The complete byte equality deliberately rejects other scripts and future contract forms.
fn reward_script(miner_pk: &[u8; 33]) -> Vec<u8> {
    locked_reward_script(720, miner_pk)
}

/// The same contract with an arbitrary height lock. The lock is the script's first
/// constant, an `Int` serialized as a zigzag VLQ (720 encodes as `a00b`).
fn locked_reward_script(lock_blocks: u32, miner_pk: &[u8; 33]) -> Vec<u8> {
    let mut zigzag = u64::from(lock_blocks) << 1;
    let mut lock = Vec::new();
    loop {
        let byte = (zigzag & 0x7f) as u8;
        zigzag >>= 7;
        if zigzag == 0 {
            lock.push(byte);
            break;
        }
        lock.push(byte | 0x80);
    }
    hex::decode(format!(
        "100204{}08cd{}ea02d192a39a8cc7a70173007301",
        hex::encode(lock),
        hex::encode(miner_pk)
    ))
    .expect("fixed reward script hex")
}

pub async fn get(
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> Result<Json<RewardBreakdown>, ApiError> {
    let result = blocking(&state, move |rd| {
        let height = super::blocks::resolve_height(rd, &raw)?;
        let mut budget = Budget::new();
        let h = rd.header_at_admitted(height, |n| budget.admit_tx_row(n))?.ok_or(ApiError::NotFound)?;
        let mut result = RewardBreakdown {
            block_id: hex::encode(h.id), height,
            basis: "unsupported", gross_reward: None, reemission_obligation: None,
            miner_subsidy: None, treasury_reward: None, lp_reward: None,
            transaction_fees: h.fees.to_string(),
            reward_box_id: None, transaction_id: None,
            note: "No supported EIP-27 emission reward was identified. Fees are separate; storage-rent income is not included.",
        };
        // A configured split-emission chain: tx 0 is
        // `[emission box, treasury, liquidity fund, miner]`, with no re-emission.
        if let Some(split) = &xp_types::chain::get().split_emission {
            result.note = "No split emission reward was identified. Fees are separate; storage-rent income is not included.";
            let page = rd.txs_in_block_page_admitted(height, None, 1, Dir::Asc, |n| budget.admit_tx_row(n))?;
            let Some((tx_id, tx)) = page.items.first() else { return Ok(result); };
            if tx.index != 0 || tx.output_count != 4 { return Ok(result); }
            let reserve = read_box(rd, &mut budget, &rd.output_id(tx, 0)?)?;
            if rd.emission_tree_hash()? != Some(reserve.tree_hash) { return Ok(result); }
            if let Some(nft) = split.emission_nft {
                if !reserve.tokens.contains(&(nft, 1)) { return Ok(result); }
            }
            let treasury = read_box(rd, &mut budget, &rd.output_id(tx, 1)?)?;
            let lp = read_box(rd, &mut budget, &rd.output_id(tx, 2)?)?;
            let id = rd.output_id(tx, 3)?;
            let miner = read_box(rd, &mut budget, &id)?;
            for (index, b) in [(0, &reserve), (1, &treasury), (2, &lp), (3, &miner)] {
                if b.tx_id != *tx_id || b.index != index || b.creation_height != height { return Ok(result); }
            }
            budget.admit_row(rd, ExpansionRow::Tree, &miner.tree_hash)?;
            let tree = rd.tree_row_checked(&miner.tree_hash, || budget.check().map_err(|_| xp_store::StoreError::ReadLimit("expansion_deadline")))?.ok_or_else(|| ApiError::Integrity("missing reward tree".into()))?;
            if tree.tree_bytes != locked_reward_script(split.miner_lock_blocks, &h.miner_pk) { return Ok(result); }
            let Some(gross) = treasury.value.checked_add(lp.value).and_then(|v| v.checked_add(miner.value)) else { return Ok(result); };
            budget.check()?;
            result.basis = "observed_split_emission";
            result.gross_reward = Some(gross.to_string());
            result.miner_subsidy = Some(miner.value.to_string());
            result.treasury_reward = Some(treasury.value.to_string());
            result.lp_reward = Some(lp.value.to_string());
            result.reward_box_id = Some(hex::encode(id));
            result.transaction_id = Some(hex::encode(tx_id));
            result.note = "Observed emission transaction outputs: miner, treasury and liquidity fund. The miner box is height-locked. Fees are separate; storage-rent income is not included.";
            return Ok(result);
        }
        // This recognizer covers the original emission phase after EIP-27 activation.
        // Re-emission withdrawals and pre-activation rewards need different evidence.
        if !(777_217..2_080_800).contains(&height) { return Ok(result); }
        let page = rd.txs_in_block_page_admitted(height, None, 1, Dir::Asc, |n| budget.admit_tx_row(n))?;
        let Some((tx_id, tx)) = page.items.first() else { return Ok(result); };
        if tx.index != 0 || tx.output_count != 2 { return Ok(result); }
        let reserve = read_box(rd, &mut budget, &rd.output_id(tx, 0)?)?;
        let nft = crate::dto::parse_id(EMISSION_NFT)?;
        if reserve.tx_id != *tx_id || reserve.index != 0 || reserve.creation_height != height || !reserve.tokens.contains(&(nft, 1)) { return Ok(result); }
        let id = rd.output_id(tx, 1)?;
        let reward = read_box(rd, &mut budget, &id)?;
        budget.admit_row(rd, ExpansionRow::Tree, &reward.tree_hash)?;
        let tree = rd.tree_row_checked(&reward.tree_hash, || budget.check().map_err(|_| xp_store::StoreError::ReadLimit("expansion_deadline")))?.ok_or_else(|| ApiError::Integrity("missing reward tree".into()))?;
        if reward.tx_id != *tx_id || reward.index != 1 || reward.creation_height != height || tree.tree_bytes != reward_script(&h.miner_pk) { return Ok(result); }
        let token = crate::dto::parse_id(REEMISSION_TOKEN)?;
        let Some((_, obligation)) = reward.tokens.iter().find(|(id, _)| *id == token) else { return Ok(result); };
        let Some(net) = reward.value.checked_sub(*obligation) else { return Ok(result); };
        budget.check()?;
        result.basis = "observed_eip27_reward_box";
        result.gross_reward = Some(reward.value.to_string());
        result.reemission_obligation = Some(obligation.to_string());
        result.miner_subsidy = Some(net.to_string());
        result.reward_box_id = Some(hex::encode(id));
        result.transaction_id = Some(hex::encode(tx_id));
        result.note = "Gross reward minus the observed EIP-27 token obligation. The obligation is paid when the reward box is spent, not necessarily in this block. Fees are separate; storage-rent income is not included.";
        Ok(result)
    }).await.map_err(|error| match error {
        ApiError::Expansion(_) => ApiError::History {
            status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            code: "reward_evidence_limit",
            detail: "Reward evidence exceeds this request's read budget.".into(),
        },
        other => other,
    })?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_script_matches_the_ergo_contract_at_720() {
        let pk = [2u8; 33];
        let expected = hex::decode(format!(
            "100204a00b08cd{}ea02d192a39a8cc7a70173007301",
            hex::encode(pk)
        ))
        .unwrap();
        assert_eq!(reward_script(&pk), expected);
        assert_eq!(locked_reward_script(720, &pk), expected);
    }

    #[test]
    fn lock_is_a_zigzag_vlq() {
        let pk = [3u8; 33];
        let lock_hex = |n: u32| {
            let script = hex::encode(locked_reward_script(n, &pk));
            let end = script.find("08cd").unwrap();
            script[6..end].to_owned()
        };
        assert_eq!(lock_hex(2_880), "802d");
        assert_eq!(lock_hex(1), "02");
        assert_eq!(lock_hex(63), "7e");
        assert_eq!(lock_hex(64), "8001");
    }
}
