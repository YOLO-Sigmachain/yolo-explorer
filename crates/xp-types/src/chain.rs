//! Chain profile: the handful of constants that differ between Ergo mainnet and a fork.
//!
//! The profile is process-wide and set at most once, at startup, before any block is
//! decoded. When it is never set, every reader gets [`ChainProfile::ergo_mainnet`], so a
//! deployment with no `[chain]` config section behaves exactly as before.
use crate::rent::RENT_PER_BYTE;
use crate::Hash32;
use std::sync::OnceLock;

const SECONDS_PER_DAY: u64 = 86_400;

/// A coinbase that pays three parties from one emission transaction:
/// `[emission box, treasury, liquidity fund, miner]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitEmission {
    /// The singleton token carried by the emission box, when it carries one. The emission
    /// box is always matched by its script as well.
    pub emission_nft: Option<Hash32>,
    /// Height lock of the miner's reward box (720 on Ergo's contract).
    pub miner_lock_blocks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainProfile {
    pub name: String,
    pub ticker: String,
    /// Network prefix of the address header byte: 0 Ergo mainnet, 16 testnet.
    pub address_prefix: u8,
    pub block_seconds: u64,
    pub rent_per_byte: u64,
    /// Every nano-coin the chain will ever have. `None` selects the built-in Ergo mainnet
    /// genesis accounting.
    pub genesis_total_nano: Option<u64>,
    pub split_emission: Option<SplitEmission>,
}

impl ChainProfile {
    pub fn ergo_mainnet() -> ChainProfile {
        ChainProfile {
            name: "ergo-mainnet".to_owned(),
            ticker: "ERG".to_owned(),
            address_prefix: 0,
            block_seconds: 120,
            rent_per_byte: RENT_PER_BYTE,
            genesis_total_nano: None,
            split_emission: None,
        }
    }

    /// Blocks in 24 hours at the target block time.
    pub fn blocks_per_day(&self) -> u32 {
        (SECONDS_PER_DAY / self.block_seconds.max(1)) as u32
    }

    /// Reject values that would make later arithmetic meaningless.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() || self.ticker.is_empty() {
            return Err("name and ticker must not be empty".into());
        }
        // The name is written to the data dir's marker file and read back trimmed, so it
        // must survive that round trip unchanged.
        if self.name.trim() != self.name || self.name.chars().any(char::is_control) {
            return Err(format!(
                "name {:?} has surrounding whitespace or control characters",
                self.name
            ));
        }
        // The header byte is `prefix + address type`; the type uses the low four bits.
        if !self.address_prefix.is_multiple_of(16) {
            return Err(format!(
                "address_prefix {} is not a multiple of 16",
                self.address_prefix
            ));
        }
        if self.block_seconds == 0 || self.block_seconds > SECONDS_PER_DAY {
            return Err(format!("block_seconds {} out of range", self.block_seconds));
        }
        // Consensus multiplies in i32; a rate that does not fit cannot be a real parameter.
        if self.rent_per_byte == 0 || self.rent_per_byte > i32::MAX as u64 {
            return Err(format!("rent_per_byte {} out of range", self.rent_per_byte));
        }
        if let Some(split) = &self.split_emission {
            if split.miner_lock_blocks == 0 {
                return Err("miner_lock_blocks must be positive".into());
            }
        }
        Ok(())
    }
}

static PROFILE: OnceLock<ChainProfile> = OnceLock::new();

/// Install the profile. Returns `false`, changing nothing, if one is already in place
/// (including the default, once anything has read it).
pub fn set(profile: ChainProfile) -> bool {
    PROFILE.set(profile).is_ok()
}

/// The active profile; Ergo mainnet unless [`set`] ran first.
pub fn get() -> &'static ChainProfile {
    PROFILE.get_or_init(ChainProfile::ergo_mainnet)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yolo() -> ChainProfile {
        ChainProfile {
            name: "yolo-testnet".into(),
            ticker: "YOLO".into(),
            address_prefix: 16,
            block_seconds: 30,
            rent_per_byte: 300_000,
            genesis_total_nano: Some(177_291_451_000_000_000),
            split_emission: Some(SplitEmission {
                emission_nft: Some([7; 32]),
                miner_lock_blocks: 2_880,
            }),
        }
    }

    #[test]
    fn default_is_ergo_mainnet() {
        let p = ChainProfile::ergo_mainnet();
        assert_eq!(p.address_prefix, 0);
        assert_eq!(p.block_seconds, 120);
        assert_eq!(p.rent_per_byte, 1_250_000);
        assert_eq!(p.blocks_per_day(), 720);
        assert!(p.genesis_total_nano.is_none() && p.split_emission.is_none());
        assert!(p.validate().is_ok());
    }

    #[test]
    fn blocks_per_day_follows_block_time() {
        assert_eq!(yolo().blocks_per_day(), 2_880);
    }

    #[test]
    fn validate_rejects_bad_values() {
        assert!(yolo().validate().is_ok());
        let bad = |f: fn(&mut ChainProfile)| {
            let mut p = yolo();
            f(&mut p);
            p.validate().is_err()
        };
        assert!(bad(|p| p.address_prefix = 17));
        assert!(bad(|p| p.block_seconds = 0));
        assert!(bad(|p| p.rent_per_byte = 0));
        assert!(bad(|p| p.rent_per_byte = i32::MAX as u64 + 1));
        assert!(bad(|p| p.ticker.clear()));
        assert!(bad(|p| p.name = "yolo-testnet ".into()));
        assert!(bad(|p| p.name = " yolo-testnet".into()));
        assert!(bad(|p| p.name = "yolo\ntestnet".into()));
        assert!(bad(|p| p.name = "yolo\ttestnet".into()));
        assert!(bad(|p| p.name = "yolo\u{7f}".into()));
        assert!(bad(|p| p
            .split_emission
            .as_mut()
            .unwrap()
            .miner_lock_blocks = 0));
    }
}
