//! The API under a configured split-emission chain profile (30 s blocks, prefix 16). The
//! profile is process-wide, so this file is its own test binary and every test installs the
//! profile before anything reads it.
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{watch, Semaphore};
use tower::ServiceExt;
use xp_api::{ApiConfig, AppState, Counters};
use xp_ingest::{IngestStatus, Mode};
use xp_store::Store;
use xp_types::chain::{ChainProfile, SplitEmission};
use xp_types::rent::RENT_PERIOD;
use xp_types::{BoxId, Hash32, HeaderId, TxId};
use xp_wire::{DecodedBlock, DecodedBox, DecodedTx};

const TOTAL: u64 = 1_000_000_000_000;
const NFT: Hash32 = [0x4e; 32];
const EMISSION: &[u8] = &[0, 8, 0xd3];
const FUND: &[u8] = &[0, 8, 0xd2];
const TREASURY: &[u8] = &[0, 8, 0xd1];
const LP: &[u8] = &[0, 8, 0xd0];

fn profile() -> ChainProfile {
    ChainProfile {
        name: "yolo-testnet".into(),
        ticker: "YOLO".into(),
        address_prefix: 16,
        block_seconds: 30,
        rent_per_byte: 300_000,
        genesis_total_nano: Some(TOTAL),
        split_emission: Some(SplitEmission {
            emission_nft: Some(NFT),
            miner_lock_blocks: 2_880,
        }),
    }
}

/// Install the YOLO profile; a no-op after the first test in this binary did it.
fn yolo() {
    xp_types::chain::set(profile());
    assert_eq!(xp_types::chain::get(), &profile());
}

fn id(n: u32) -> Hash32 {
    let mut id = [0x5a; 32];
    id[..4].copy_from_slice(&n.to_be_bytes());
    id
}

fn output(n: u32, tree: &[u8], value: u64, tokens: Vec<(Hash32, u64)>) -> DecodedBox {
    DecodedBox {
        id: BoxId(id(n)),
        value,
        tree_bytes: tree.to_vec(),
        tree_hash: xp_wire::tree_hash(tree),
        creation_height: 0,
        tx_id: TxId([0; 32]),
        index: 0,
        tokens,
        registers_json: "{}".into(),
        id_verified: false,
        size: 100,
    }
}

fn tx(n: u32, height: u32, inputs: &[u32], mut outputs: Vec<DecodedBox>) -> DecodedTx {
    for (index, output) in outputs.iter_mut().enumerate() {
        output.tx_id = TxId(id(n));
        output.index = index as u16;
        output.creation_height = height;
    }
    DecodedTx {
        id: TxId(id(n)),
        inputs: inputs.iter().map(|n| BoxId(id(*n))).collect(),
        data_inputs: vec![],
        outputs,
        size: 100,
    }
}

fn template() -> DecodedBlock {
    xp_wire::decode_block(include_str!("../../../tests/fixtures/blocks/1866000.json")).unwrap()
}

fn block(height: u32, parent: Hash32, txs: Vec<DecodedTx>) -> DecodedBlock {
    let mut b = template();
    b.header.height = height;
    b.header.id = HeaderId(id(1_000_000 + height));
    b.header.parent_id = HeaderId(parent);
    b.header.timestamp = 1_800_000_000_000 + u64::from(height) * 30_000;
    b.txs = txs;
    b
}

/// Ergo's delayed miner reward contract with a 2,880-block lock (`802d`).
fn miner_script(pk: &[u8; 33]) -> Vec<u8> {
    hex::decode(format!(
        "100204802d08cd{}ea02d192a39a8cc7a70173007301",
        hex::encode(pk)
    ))
    .unwrap()
}

fn router(store: Arc<Store>) -> Router {
    let cfg = ApiConfig {
        per_second: 0,
        ..Default::default()
    };
    let (_, status) = watch::channel(IngestStatus {
        source_observed_at_ms: None,
        source_error: None,
        indexed: None,
        best: 0,
        mode: Mode::Tip,
        source: "test".into(),
        halted: None,
        stalled: None,
    });
    xp_api::router(
        AppState {
            store,
            status,
            counters: Arc::new(Counters::default()),
            read_permits: Arc::new(Semaphore::new(cfg.max_inflight_reads as usize)),
        },
        &cfg,
    )
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

const RESERVE: u64 = TOTAL - 1_000;

fn genesis(fund: u64) -> Vec<DecodedBox> {
    vec![
        output(1, EMISSION, RESERVE, vec![(NFT, 1)]),
        output(2, FUND, fund, vec![]),
    ]
}

/// Height 1 is a real emission split plus a lookalike payment to the emission script;
/// height 2 is a split-shaped transaction 0 whose "reserve" carries no NFT.
fn emission_chain(fund: u64) -> (tempfile::TempDir, Arc<Store>) {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("yolo.redb")).unwrap());
    store.seed_genesis(&genesis(fund)).unwrap();
    let pk = template().header.miner_pk;
    let b1 = block(
        1,
        [0; 32],
        vec![
            tx(
                10,
                1,
                &[1],
                vec![
                    output(11, EMISSION, RESERVE - 75_000, vec![(NFT, 1)]),
                    output(12, TREASURY, 7_500, vec![]),
                    output(13, LP, 3_750, vec![]),
                    output(14, &miner_script(&pk), 63_750, vec![]),
                ],
            ),
            // Anyone can pay to the emission script.
            tx(20, 1, &[2], vec![output(21, EMISSION, fund, vec![])]),
        ],
    );
    let b2 = block(
        2,
        b1.header.id.0,
        vec![tx(
            30,
            2,
            &[21],
            vec![
                output(31, EMISSION, fund - 300, vec![]),
                output(32, TREASURY, 100, vec![]),
                output(33, LP, 100, vec![]),
                output(34, &miner_script(&pk), 100, vec![]),
            ],
        )],
    );
    store.apply_batch(&[b1, b2], true).unwrap();
    (dir, store)
}

#[tokio::test]
async fn split_emission_reward_is_recognised_and_a_lookalike_is_not() {
    yolo();
    let (_dir, store) = emission_chain(1_000);
    let app = router(store);
    let (status, reward) = get(&app, "/v1/blocks/1/rewards").await;
    assert_eq!(status, StatusCode::OK, "{reward}");
    assert_eq!(reward["basis"], "observed_split_emission");
    assert_eq!(reward["gross_reward"], "75000");
    assert_eq!(reward["miner_subsidy"], "63750");
    assert_eq!(reward["treasury_reward"], "7500");
    assert_eq!(reward["lp_reward"], "3750");
    assert_eq!(reward["reward_box_id"], hex::encode(id(14)));
    assert_eq!(reward["transaction_id"], hex::encode(id(10)));
    let (status, fake) = get(&app, "/v1/blocks/2/rewards").await;
    assert_eq!(status, StatusCode::OK, "{fake}");
    assert_eq!(fake["basis"], "unsupported");
    assert!(fake["gross_reward"].is_null());
}

#[tokio::test]
async fn configured_supply_reads_the_nft_box_not_the_emission_script_balance() {
    yolo();
    let (_dir, store) = emission_chain(1_000);
    let app = router(store);
    let (status, supply) = get(&app, "/v1/supply").await;
    assert_eq!(status, StatusCode::OK, "{supply}");
    assert_eq!(supply["genesis_total_nano"], TOTAL.to_string());
    // The lookalike box at height 2 still sits on the emission script; it is not reserve.
    assert_eq!(
        supply["emission_remaining_nano"],
        (RESERVE - 75_000).to_string()
    );
    assert_eq!(supply["outside_emission_nano"], "76000");
    assert_eq!(supply["emitted_nano"], "76000");
    assert_eq!(supply["complete"], true);
}

#[tokio::test]
async fn configured_supply_is_incomplete_when_genesis_does_not_add_up() {
    yolo();
    let (_dir, store) = emission_chain(1_001);
    let (status, supply) = get(&router(store), "/v1/supply").await;
    assert_eq!(status, StatusCode::OK, "{supply}");
    assert_eq!(supply["genesis_total_nano"], TOTAL.to_string());
    assert_eq!(supply["complete"], false);
}

#[tokio::test]
async fn configured_supply_on_an_index_without_a_recorded_genesis_sum() {
    yolo();
    // An index seeded before the sum was recorded cannot be checked: same answer as before.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("yolo.redb");
    {
        let store = Store::open(&path).unwrap();
        store.seed_genesis(&genesis(1_001)).unwrap();
    }
    {
        let db = redb::Database::create(&path).unwrap();
        let tx = db.begin_write().unwrap();
        assert!(tx
            .open_table(xp_store::tables::META)
            .unwrap()
            .remove(xp_store::tables::META_GENESIS_VALUE_SUM)
            .unwrap()
            .is_some());
        tx.commit().unwrap();
    }
    let store = Arc::new(Store::open(&path).unwrap());
    let (status, supply) = get(&router(store), "/v1/supply").await;
    assert_eq!(status, StatusCode::OK, "{supply}");
    assert_eq!(supply["emission_remaining_nano"], RESERVE.to_string());
    assert_eq!(supply["complete"], true);
}

#[tokio::test]
async fn rent_schedule_windows_are_days_of_30_second_blocks() {
    yolo();
    let tip = RENT_PERIOD - 1;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("rent.redb")).unwrap());
    store.seed_for_tests(tip - 1, id(99_000)).unwrap();
    // Maturity is creation + RENT_PERIOD; the 90-day window ends at tip + 259,200.
    let boxes = [0, 64_800, 259_199, 259_200]
        .into_iter()
        .enumerate()
        .map(|(n, creation)| {
            let mut b = output(500 + n as u32, FUND, 1_000_000, vec![]);
            b.creation_height = creation;
            b.tx_id = TxId(id(90_000));
            b.index = n as u16;
            b
        })
        .collect();
    let mut b = block(tip, id(99_000), vec![]);
    b.header.id = HeaderId(id(99_001));
    b.txs = vec![DecodedTx {
        id: TxId(id(90_000)),
        inputs: vec![BoxId(id(99_999))],
        data_inputs: vec![],
        outputs: boxes,
        size: 100,
    }];
    store.apply_batch(&[b], true).unwrap();
    let app = router(store);
    for (window, span, count) in [
        ("24h", 2_880, 1),
        ("7d", 20_160, 1),
        ("30d", 86_400, 2),
        ("90d", 259_200, 3),
    ] {
        let (status, page) = get(&app, &format!("/v1/rent/schedule?window={window}")).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        let context = &page["schedule_context"];
        assert_eq!(context["to_height"], tip + span, "{window}");
        assert_eq!(context["target_block_seconds"], 30);
        assert_eq!(context["rent_factor"], 300_000);
        assert_eq!(page["items"].as_array().unwrap().len(), count, "{window}");
        assert_eq!(
            page["items"][0]["estimated_maturity_ms"],
            context["anchor_timestamp_ms"].as_u64().unwrap() + 30_000
        );
    }
}

#[tokio::test]
async fn network_summary_covers_a_full_day_of_fixture_sized_headers() {
    yolo();
    let template = template();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&dir.path().join("summary.redb")).unwrap());
    store
        .seed_for_tests(template.header.height - 1, template.header.parent_id.0)
        .unwrap();
    let mut parent = template.header.parent_id;
    let mut blocks = Vec::new();
    for index in 0..2_880u32 {
        let mut header = template.header.clone();
        header.height += index;
        header.timestamp += u64::from(index) * 30_000;
        header.id.0[..4].copy_from_slice(&index.to_be_bytes());
        header.parent_id = parent;
        parent = header.id;
        blocks.push(DecodedBlock {
            header,
            txs: vec![],
            size: 100,
        });
    }
    // Fixture-sized headers: one day of them exceeds Ergo's per-request decode allowance.
    assert!(template.header.raw_json.len() * 2_880 > 2 * 1024 * 1024);
    store.apply_batch(&blocks, true).unwrap();
    let (status, summary) = get(&router(store), "/v1/network/summary").await;
    assert_eq!(status, StatusCode::OK, "{summary}");
    assert_eq!(summary["requested_blocks"], 2_880);
    assert_eq!(summary["block_count"], 2_880);
    assert_eq!(summary["to_height"], template.header.height + 2_879);
}
