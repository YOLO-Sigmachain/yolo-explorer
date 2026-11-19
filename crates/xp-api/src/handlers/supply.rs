//! Gross allocation outside the original emission reserve, explicitly not circulation.
use crate::dto::SupplyDto;
use crate::{blocking, ApiError, AppState};
use axum::extract::State;
use axum::Json;
use xp_store::read::Dir;
use xp_types::GENESIS_TOTAL_NANO;

pub async fn supply(State(state): State<AppState>) -> Result<Json<SupplyDto>, ApiError> {
    Ok(Json(
        blocking(&state, move |rd| {
            let mut dto = SupplyDto {
                indexed_height: rd.indexed_height()?,
                emission_remaining_nano: None,
                emitted_nano: None,
                outside_emission_nano: None,
                circulating_nano: None,
                definition: "genesis_allocation_minus_original_emission_reserve",
                genesis_total_nano: GENESIS_TOTAL_NANO.to_string(),
                complete: false,
            };
            // A configured chain: the genesis total comes from the profile and the emission
            // script is the one recorded when the chain-spec boxes were seeded.
            let profile = xp_types::chain::get();
            if let Some(total) = profile.genesis_total_nano {
                dto.genesis_total_nano = total.to_string();
                if !rd.full_history()? {
                    return Ok(dto);
                }
                // The configured total must be what genesis actually seeded. A store seeded
                // before the sum was recorded cannot be checked and is taken as configured.
                if rd
                    .genesis_value_sum()?
                    .is_some_and(|sum| sum != u128::from(total))
                {
                    return Ok(dto);
                }
                let Some(tree) = rd.emission_tree_hash()? else {
                    return Ok(dto);
                };
                let nft = profile
                    .split_emission
                    .as_ref()
                    .and_then(|split| split.emission_nft);
                let remaining = match nft {
                    // Anyone can pay to the emission script; only the box holding the NFT
                    // is the reserve. None unspent means it was burned: nothing remains.
                    Some(nft) => rd
                        .token_boxes(&nft, true, None, 1, Dir::Asc)?
                        .items
                        .first()
                        .map_or(0, |(_, b)| b.value),
                    None => {
                        rd.balance(&tree)?
                            .ok_or_else(|| ApiError::Integrity("missing emission balance".into()))?
                            .nano
                    }
                };
                let outside = total.checked_sub(remaining).ok_or_else(|| {
                    ApiError::Integrity("emission balance exceeds configured genesis total".into())
                })?;
                dto.emission_remaining_nano = Some(remaining.to_string());
                dto.outside_emission_nano = Some(outside.to_string());
                dto.emitted_nano = dto.outside_emission_nano.clone();
                dto.complete = true;
                return Ok(dto);
            }
            if !rd.mainnet_genesis()? {
                return Ok(dto);
            }
            let tree = rd
                .emission_tree_hash()?
                .ok_or_else(|| ApiError::Integrity("missing emission tree".into()))?;
            let genesis_id: [u8; 32] = hex::decode(xp_store::read::MAINNET_GENESIS[0].0)
                .expect("static genesis id")
                .try_into()
                .expect("static width");
            let genesis_tree = rd
                .box_by_id(&genesis_id)?
                .ok_or_else(|| ApiError::Integrity("missing genesis emission box".into()))?
                .tree_hash;
            if tree != genesis_tree {
                return Err(ApiError::Integrity(
                    "incorrect emission tree metadata".into(),
                ));
            }
            // Balances are retained even at zero; a missing row is not exhaustion.
            let remaining = rd
                .balance(&tree)?
                .ok_or_else(|| ApiError::Integrity("missing emission balance".into()))?
                .nano;
            let outside = GENESIS_TOTAL_NANO.checked_sub(remaining).ok_or_else(|| {
                ApiError::Integrity("emission balance exceeds genesis allocation".into())
            })?;
            dto.emission_remaining_nano = Some(remaining.to_string());
            dto.outside_emission_nano = Some(outside.to_string());
            dto.emitted_nano = dto.outside_emission_nano.clone();
            dto.complete = true;
            Ok(dto)
        })
        .await?,
    ))
}
