//! TOML configuration for the `explorer` binary.

use serde::Deserialize;
use std::path::PathBuf;

fn default_register_index_ceiling() -> Option<u64> {
    xp_store::DEFAULT_REGISTER_INDEX_CEILING
}

fn default_poll_ms() -> u64 {
    xp_ingest::IngestConfig::default().poll_ms
}
fn default_bulk_batch() -> usize {
    xp_ingest::IngestConfig::default().bulk_batch
}
fn default_bulk_concurrency() -> usize {
    xp_ingest::IngestConfig::default().bulk_concurrency
}
fn default_durable_every() -> u32 {
    xp_ingest::IngestConfig::default().durable_every
}
fn default_tip_lag_for_bulk() -> u32 {
    xp_ingest::IngestConfig::default().tip_lag_for_bulk
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    #[serde(default = "default_register_index_ceiling")]
    pub register_index_ceiling: Option<u64>,
    pub bind: String,
    pub source: SourceConfig,
    #[serde(default)]
    pub ingest: IngestSection,
    #[serde(default)]
    pub api: ApiSection,
    /// Absent means Ergo mainnet.
    #[serde(default)]
    pub chain: Option<ChainSection>,
}

/// Constants of a chain other than Ergo mainnet. See `xp_types::chain`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainSection {
    pub name: String,
    pub ticker: String,
    pub address_prefix: u8,
    pub block_seconds: u64,
    pub rent_per_byte: u64,
    #[serde(default)]
    pub genesis_total_nano: Option<u64>,
    /// Setting this selects the split-emission reward layout.
    #[serde(default)]
    pub miner_lock_blocks: Option<u32>,
    #[serde(default)]
    pub emission_nft: Option<String>,
}

impl ChainSection {
    fn profile(&self) -> anyhow::Result<xp_types::chain::ChainProfile> {
        let emission_nft = match &self.emission_nft {
            None => None,
            Some(id) => Some(
                hex::decode(id)
                    .ok()
                    .and_then(|v| <[u8; 32]>::try_from(v).ok())
                    .ok_or_else(|| anyhow::anyhow!("[chain] emission_nft is not 32-byte hex"))?,
            ),
        };
        let split_emission = match self.miner_lock_blocks {
            // Without the NFT any transaction-0 shaped like the emission split, paying the
            // emission script, would be reported as the block's reward.
            Some(_) if emission_nft.is_none() => {
                anyhow::bail!("[chain] miner_lock_blocks needs emission_nft")
            }
            Some(miner_lock_blocks) => Some(xp_types::chain::SplitEmission {
                emission_nft,
                miner_lock_blocks,
            }),
            None if emission_nft.is_some() => {
                anyhow::bail!("[chain] emission_nft needs miner_lock_blocks")
            }
            None => None,
        };
        let profile = xp_types::chain::ChainProfile {
            name: self.name.clone(),
            ticker: self.ticker.clone(),
            address_prefix: self.address_prefix,
            block_seconds: self.block_seconds,
            rent_per_byte: self.rent_per_byte,
            genesis_total_nano: self.genesis_total_nano,
            split_emission,
        };
        profile
            .validate()
            .map_err(|e| anyhow::anyhow!("[chain] {e}"))?;
        Ok(profile)
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiSection {
    pub metrics_allowlist: Vec<String>,
    pub max_inflight_reads: u32,
    pub trusted_proxies: Vec<String>,
    pub rate_limit: RateLimitSection,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RateLimitSection {
    pub per_second: u32,
    pub burst: u32,
    pub allowlist: Vec<String>,
}

impl Default for ApiSection {
    fn default() -> ApiSection {
        ApiSection {
            metrics_allowlist: Vec::new(),
            max_inflight_reads: 32,
            trusted_proxies: vec!["127.0.0.1".into(), "::1".into()],
            rate_limit: RateLimitSection::default(),
        }
    }
}

impl Default for RateLimitSection {
    fn default() -> RateLimitSection {
        RateLimitSection {
            per_second: 10,
            burst: 30,
            allowlist: Vec::new(),
        }
    }
}

impl TryFrom<&ApiSection> for xp_api::ApiConfig {
    type Error = String;
    fn try_from(s: &ApiSection) -> Result<xp_api::ApiConfig, String> {
        Ok(xp_api::ApiConfig {
            metrics_allowlist: xp_api::Allowlist::parse(&s.metrics_allowlist)
                .map_err(|e| format!("[api] metrics_allowlist: {e}"))?,
            per_second: s.rate_limit.per_second,
            burst: s.rate_limit.burst,
            allowlist: xp_api::Allowlist::parse(&s.rate_limit.allowlist)
                .map_err(|e| format!("[api.rate_limit] allowlist: {e}"))?,
            trusted_proxies: xp_api::Allowlist::parse(&s.trusted_proxies)
                .map_err(|e| format!("[api] trusted_proxies: {e}"))?,
            max_inflight_reads: s.max_inflight_reads.max(1),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct SourceConfig {
    pub kind: String,
    pub url: String,
    /// Optional second node, used *only* to fetch block bodies `url` announces but will not
    /// serve. Unset (the default) means no fallback and no behaviour change.
    #[serde(default)]
    pub fallback_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IngestSection {
    #[serde(default = "default_poll_ms")]
    pub poll_ms: u64,
    #[serde(default = "default_bulk_batch")]
    pub bulk_batch: usize,
    #[serde(default = "default_bulk_concurrency")]
    pub bulk_concurrency: usize,
    #[serde(default = "default_durable_every")]
    pub durable_every: u32,
    #[serde(default = "default_tip_lag_for_bulk")]
    pub tip_lag_for_bulk: u32,
}

impl Default for IngestSection {
    fn default() -> IngestSection {
        let d = xp_ingest::IngestConfig::default();
        IngestSection {
            poll_ms: d.poll_ms,
            bulk_batch: d.bulk_batch,
            bulk_concurrency: d.bulk_concurrency,
            durable_every: d.durable_every,
            tip_lag_for_bulk: d.tip_lag_for_bulk,
        }
    }
}

impl From<&IngestSection> for xp_ingest::IngestConfig {
    fn from(s: &IngestSection) -> xp_ingest::IngestConfig {
        xp_ingest::IngestConfig {
            poll_ms: s.poll_ms,
            bulk_batch: s.bulk_batch,
            bulk_concurrency: s.bulk_concurrency,
            durable_every: s.durable_every,
            tip_lag_for_bulk: s.tip_lag_for_bulk,
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> anyhow::Result<Config> {
        let cfg: Config = toml::from_str(text)?;
        if cfg.source.kind != "rust_node" {
            anyhow::bail!(
                "unsupported [source] kind {:?}: only \"rust_node\" is supported",
                cfg.source.kind
            );
        }
        Ok(cfg)
    }

    /// The configured chain, or Ergo mainnet when there is no `[chain]` section.
    pub fn chain_profile(&self) -> anyhow::Result<xp_types::chain::ChainProfile> {
        match &self.chain {
            Some(section) => section.profile(),
            None => Ok(xp_types::chain::ChainProfile::ergo_mainnet()),
        }
    }
}

/// Addresses are written into the index with the chain's prefix, so an index built for
/// one chain must never be served as another. The chain name and address prefix are kept
/// beside the store (`<name>\nprefix=<n>\n`) and checked on every start. A store with no
/// marker predates this check and is Ergo mainnet. A one-line marker (name only) predates
/// the prefix line: it means prefix 0 for "ergo-mainnet"; for any other name the prefix is
/// unknown, so the configured one is accepted once and the marker rewritten in full.
pub fn check_chain_marker(
    data_dir: &std::path::Path,
    chain: &str,
    prefix: u8,
) -> anyhow::Result<()> {
    let marker = data_dir.join("chain.txt");
    // (name, prefix, marker is already in the two-line form)
    let recorded: Option<(String, Option<u8>, bool)> = match std::fs::read_to_string(&marker) {
        Ok(text) => {
            let mut lines = text.lines();
            let name = lines.next().unwrap_or("").trim().to_owned();
            match lines.next().map(str::trim) {
                None | Some("") => {
                    let prefix = (name == "ergo-mainnet").then_some(0);
                    Some((name, prefix, false))
                }
                Some(line) => {
                    let value = line
                        .strip_prefix("prefix=")
                        .and_then(|n| n.parse::<u8>().ok())
                        .ok_or_else(|| {
                            anyhow::anyhow!("{}: unreadable line {line:?}", marker.display())
                        })?;
                    Some((name, Some(value), true))
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if data_dir.join("explorer.redb").exists() {
                Some(("ergo-mainnet".to_owned(), Some(0), false))
            } else {
                None
            }
        }
        Err(e) => return Err(anyhow::anyhow!("reading {}: {e}", marker.display())),
    };
    match recorded {
        Some((recorded, _, _)) if recorded != chain => anyhow::bail!(
            "data_dir {} holds an index for chain {recorded:?} but the config says {chain:?}; \
             use another data_dir or delete the index",
            data_dir.display()
        ),
        Some((_, Some(recorded), _)) if recorded != prefix => anyhow::bail!(
            "data_dir {} holds an index with address prefix {recorded} but the config says \
             {prefix}; use another data_dir or delete the index",
            data_dir.display()
        ),
        Some((_, Some(_), true)) => Ok(()),
        _ => std::fs::write(&marker, format!("{chain}\nprefix={prefix}\n"))
            .map_err(|e| anyhow::anyhow!("writing {}: {e}", marker.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../explorer.example.toml");

    #[test]
    fn register_ceiling_defaults_and_parses_override() {
        let cfg = Config::parse(EXAMPLE).unwrap();
        assert_eq!(cfg.register_index_ceiling, None);
        let cfg = Config::parse(&format!("register_index_ceiling = 42\n{EXAMPLE}")).unwrap();
        assert_eq!(cfg.register_index_ceiling, Some(42));
        let cfg = Config::parse(&format!("register_index_ceiling = 0\n{EXAMPLE}")).unwrap();
        assert_eq!(cfg.register_index_ceiling, Some(0));
        assert!(Config::parse(&format!("register_index_ceiling = -1\n{EXAMPLE}")).is_err());
    }

    #[test]
    fn parses_example_toml() {
        let cfg = Config::parse(EXAMPLE).expect("example config should parse");
        assert_eq!(cfg.data_dir, PathBuf::from("./data"));
        assert_eq!(cfg.bind, "127.0.0.1:8090");
        assert_eq!(cfg.source.kind, "rust_node");
        assert_eq!(cfg.source.url, "http://127.0.0.1:9063");
        assert_eq!(
            cfg.source.fallback_url, None,
            "the example's fallback_url is commented out: no fallback by default"
        );
        assert_eq!(cfg.ingest.poll_ms, 500);
        assert_eq!(cfg.ingest.bulk_batch, 64);
        assert_eq!(cfg.ingest.bulk_concurrency, 8);
        assert_eq!(cfg.ingest.durable_every, 256);
        assert_eq!(cfg.ingest.tip_lag_for_bulk, 64);
    }

    #[test]
    fn defaults_applied_when_ingest_omitted() {
        let text = r#"
            data_dir = "./data"
            bind = "127.0.0.1:8090"
            [source]
            kind = "rust_node"
            url = "http://127.0.0.1:9063"
        "#;
        let cfg = Config::parse(text).expect("config without [ingest] should parse");
        let default = xp_ingest::IngestConfig::default();
        assert_eq!(cfg.ingest.poll_ms, default.poll_ms);
        assert_eq!(cfg.ingest.bulk_batch, default.bulk_batch);
        assert_eq!(cfg.ingest.bulk_concurrency, default.bulk_concurrency);
        assert_eq!(cfg.ingest.durable_every, default.durable_every);
        assert_eq!(cfg.ingest.tip_lag_for_bulk, default.tip_lag_for_bulk);
    }

    #[test]
    fn fallback_url_is_optional_and_parsed_when_present() {
        let text = r#"
            data_dir = "./data"
            bind = "127.0.0.1:8090"
            [source]
            kind = "rust_node"
            url = "http://127.0.0.1:9063"
            fallback_url = "https://node.ergo.watch"
        "#;
        let cfg = Config::parse(text).expect("config with fallback_url should parse");
        assert_eq!(
            cfg.source.fallback_url.as_deref(),
            Some("https://node.ergo.watch")
        );
    }

    #[test]
    fn api_section_defaults_when_absent() {
        let cfg = Config::parse(
            r#"
        data_dir = "/tmp/x"
        bind = "127.0.0.1:1"
        [source]
        kind = "rust_node"
        url = "http://127.0.0.1:9053"
    "#,
        )
        .unwrap();
        let api = xp_api::ApiConfig::try_from(&cfg.api).unwrap();
        assert_eq!(
            (api.per_second, api.burst, api.max_inflight_reads),
            (10, 30, 32)
        );
        assert!(api.trusted_proxies.contains("127.0.0.1".parse().unwrap()));
        assert!(!api.allowlist.contains("1.1.1.1".parse().unwrap()));
    }

    #[test]
    fn api_section_parses_and_rejects_bad_cidr() {
        let good = Config::parse(
            r#"
        data_dir = "/tmp/x"
        bind = "127.0.0.1:1"
        [source]
        kind = "rust_node"
        url = "http://127.0.0.1:9053"
        [api]
        max_inflight_reads = 4
        trusted_proxies = ["10.0.0.1"]
        [api.rate_limit]
        per_second = 2
        burst = 5
        allowlist = ["203.0.113.0/24", "2001:db8::/32"]
    "#,
        )
        .unwrap();
        let api = xp_api::ApiConfig::try_from(&good.api).unwrap();
        assert_eq!(
            (api.per_second, api.burst, api.max_inflight_reads),
            (2, 5, 4)
        );
        assert!(api.allowlist.contains("203.0.113.9".parse().unwrap()));
        assert!(api.trusted_proxies.contains("10.0.0.1".parse().unwrap()));
        assert!(!api.trusted_proxies.contains("127.0.0.1".parse().unwrap()));

        let bad = Config::parse(
            r#"
        data_dir = "/tmp/x"
        bind = "127.0.0.1:1"
        [source]
        kind = "rust_node"
        url = "http://127.0.0.1:9053"
        [api.rate_limit]
        allowlist = ["1.2.3.4/40"]
    "#,
        )
        .unwrap();
        let err = xp_api::ApiConfig::try_from(&bad.api).unwrap_err();
        assert!(err.contains("1.2.3.4/40"), "{err}");
    }

    #[test]
    fn unknown_source_kind_rejected() {
        let text = r#"
            data_dir = "./data"
            bind = "127.0.0.1:8090"
            [source]
            kind = "scala_node"
            url = "http://127.0.0.1:9053"
        "#;
        let err = Config::parse(text).expect_err("unknown source kind must be rejected");
        assert!(err.to_string().contains("scala_node"));
    }

    #[test]
    fn api_section_rejects_unknown_keys() {
        let err = Config::parse(
            r#"
        data_dir = "/tmp/x"
        bind = "127.0.0.1:1"
        [source]
        kind = "rust_node"
        url = "http://127.0.0.1:9053"
        [api]
        max_inflight_read = 4
    "#,
        )
        .expect_err("typo in [api] must fail to parse");
        assert!(format!("{err:#}").contains("max_inflight_read"), "{err:#}");
    }

    const BASE: &str = r#"
        data_dir = "./data"
        bind = "127.0.0.1:8090"
        [source]
        kind = "rust_node"
        url = "http://127.0.0.1:9053"
    "#;

    #[test]
    fn no_chain_section_is_ergo_mainnet() {
        let cfg = Config::parse(BASE).unwrap();
        assert_eq!(
            cfg.chain_profile().unwrap(),
            xp_types::chain::ChainProfile::ergo_mainnet()
        );
    }

    #[test]
    fn chain_section_builds_a_split_emission_profile() {
        let text = format!(
            "{BASE}\n[chain]\nname = \"yolo-testnet\"\nticker = \"YOLO\"\naddress_prefix = 16\n\
             block_seconds = 30\nrent_per_byte = 300000\n\
             genesis_total_nano = 177291451000000000\nminer_lock_blocks = 2880\n\
             emission_nft = \"{}\"\n",
            "ab".repeat(32)
        );
        let p = Config::parse(&text).unwrap().chain_profile().unwrap();
        assert_eq!(p.name, "yolo-testnet");
        assert_eq!(p.blocks_per_day(), 2_880);
        assert_eq!(p.genesis_total_nano, Some(177_291_451_000_000_000));
        let split = p.split_emission.unwrap();
        assert_eq!(split.miner_lock_blocks, 2_880);
        assert_eq!(split.emission_nft, Some([0xab; 32]));
    }

    #[test]
    fn bad_chain_sections_are_rejected() {
        let with = |extra: &str| {
            let text = format!(
                "{BASE}\n[chain]\nname = \"x\"\nticker = \"X\"\naddress_prefix = 16\n\
                 block_seconds = 30\nrent_per_byte = 300000\n{extra}\n"
            );
            Config::parse(&text).and_then(|c| c.chain_profile())
        };
        assert!(with("").is_ok());
        assert!(with("emission_nft = \"zz\"\nminer_lock_blocks = 2880").is_err());
        assert!(with(&format!("emission_nft = \"{}\"", "ab".repeat(32))).is_err());
        assert!(with("miner_lock_blocks = 0").is_err());
        assert!(with(&format!(
            "emission_nft = \"{}\"\nminer_lock_blocks = 0",
            "ab".repeat(32)
        ))
        .is_err());
        let err = with("miner_lock_blocks = 2880").unwrap_err();
        assert!(format!("{err:#}").contains("emission_nft"), "{err:#}");
        assert!(with(&format!(
            "emission_nft = \"{}\"\nminer_lock_blocks = 2880",
            "ab".repeat(32)
        ))
        .is_ok());
        assert!(with("unknown_key = 1").is_err());
        let text = format!(
            "{BASE}\n[chain]\nname = \"x\"\nticker = \"X\"\naddress_prefix = 17\n\
             block_seconds = 30\nrent_per_byte = 300000\n"
        );
        assert!(Config::parse(&text).unwrap().chain_profile().is_err());
    }

    #[test]
    fn chain_marker_binds_a_data_dir_to_one_chain() {
        let dir = std::env::temp_dir().join(format!("xp-chain-marker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("chain.txt");
        let read = || std::fs::read_to_string(&marker).unwrap();
        // Fresh directory: any chain is accepted and recorded with its prefix.
        check_chain_marker(&dir, "yolo-testnet", 16).unwrap();
        assert_eq!(read(), "yolo-testnet\nprefix=16\n");
        check_chain_marker(&dir, "yolo-testnet", 16).unwrap();
        assert!(check_chain_marker(&dir, "ergo-mainnet", 0).is_err());
        // Same name, different prefix: the stored address strings would be wrong.
        assert!(check_chain_marker(&dir, "yolo-testnet", 32).is_err());
        assert_eq!(read(), "yolo-testnet\nprefix=16\n");
        // A store with no marker predates the check and is Ergo mainnet.
        std::fs::remove_file(&marker).unwrap();
        std::fs::write(dir.join("explorer.redb"), b"x").unwrap();
        assert!(check_chain_marker(&dir, "yolo-testnet", 16).is_err());
        assert!(check_chain_marker(&dir, "ergo-mainnet", 16).is_err());
        check_chain_marker(&dir, "ergo-mainnet", 0).unwrap();
        assert_eq!(read(), "ergo-mainnet\nprefix=0\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn one_line_markers_from_before_the_prefix_line() {
        let dir = std::env::temp_dir().join(format!("xp-chain-marker-old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("explorer.redb"), b"x").unwrap();
        let marker = dir.join("chain.txt");
        let read = || std::fs::read_to_string(&marker).unwrap();
        // Ergo mainnet: the old marker means prefix 0.
        std::fs::write(&marker, "ergo-mainnet\n").unwrap();
        assert!(check_chain_marker(&dir, "ergo-mainnet", 16).is_err());
        assert_eq!(read(), "ergo-mainnet\n");
        check_chain_marker(&dir, "ergo-mainnet", 0).unwrap();
        assert_eq!(read(), "ergo-mainnet\nprefix=0\n");
        // Any other name: prefix unknown, accepted once and then pinned.
        std::fs::write(&marker, "yolo-devnet\n").unwrap();
        assert!(check_chain_marker(&dir, "yolo-testnet", 16).is_err());
        check_chain_marker(&dir, "yolo-devnet", 16).unwrap();
        assert_eq!(read(), "yolo-devnet\nprefix=16\n");
        check_chain_marker(&dir, "yolo-devnet", 16).unwrap();
        assert!(check_chain_marker(&dir, "yolo-devnet", 32).is_err());
        // A second line that is not a prefix is refused, not guessed at.
        std::fs::write(&marker, "yolo-devnet\nsomething\n").unwrap();
        assert!(check_chain_marker(&dir, "yolo-devnet", 16).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn chain_name_with_surrounding_space_is_rejected() {
        let text = format!(
            "{BASE}\n[chain]\nname = \"yolo-testnet \"\nticker = \"YOLO\"\naddress_prefix = 16\n\
             block_seconds = 30\nrent_per_byte = 300000\n"
        );
        assert!(Config::parse(&text).unwrap().chain_profile().is_err());
    }
}
