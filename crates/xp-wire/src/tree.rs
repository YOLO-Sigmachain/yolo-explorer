use blake2::{digest::consts::U32, Blake2b, Digest};
use ergo_lib::ergotree_ir::chain::address::{Address, AddressEncoder, NetworkPrefix};
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use xp_types::{Hash32, TreeHash};

use crate::WireError;

pub fn blake2b256(data: &[u8]) -> Hash32 {
    let mut h = Blake2b::<U32>::new();
    h.update(data);
    h.finalize().into()
}

pub fn tree_hash(tree_bytes: &[u8]) -> TreeHash {
    TreeHash(blake2b256(tree_bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TreeKind {
    P2pk = 0,
    P2s = 1,
    Other = 2,
}

#[derive(Debug, Clone)]
pub struct TreeInfo {
    pub template_hash: Hash32,
    pub address: String,
    pub kind: TreeKind,
}

pub fn template_hash_of(tree_bytes: &[u8]) -> Result<Hash32, WireError> {
    let tree =
        ErgoTree::sigma_parse_bytes(tree_bytes).map_err(|e| WireError::Tree(e.to_string()))?;
    let tmpl = tree
        .template_bytes()
        .map_err(|e| WireError::Tree(e.to_string()))?;
    Ok(blake2b256(&tmpl))
}

pub fn tree_info(tree_bytes: &[u8]) -> Result<TreeInfo, WireError> {
    let tree =
        ErgoTree::sigma_parse_bytes(tree_bytes).map_err(|e| WireError::Tree(e.to_string()))?;
    let tmpl = tree
        .template_bytes()
        .map_err(|e| WireError::Tree(e.to_string()))?;
    let addr =
        Address::recreate_from_ergo_tree(&tree).map_err(|e| WireError::Tree(e.to_string()))?;
    let kind = match &addr {
        Address::P2Pk(_) => TreeKind::P2pk,
        Address::P2S(_) => TreeKind::P2s,
        _ => TreeKind::Other,
    };
    let address = encode_address(xp_types::chain::get().address_prefix, &addr);
    Ok(TreeInfo {
        template_hash: blake2b256(&tmpl),
        address,
        kind,
    })
}

const CHECKSUM_LEN: usize = 4;

/// Replace the network prefix of a base58 address whose current prefix is `from`, and
/// recompute the checksum. The header byte is `prefix + address type` with the type in the
/// low four bits; the checksum is the first four bytes of blake2b256 over what precedes it.
fn reprefix(address: &str, from: u8, to: u8) -> Result<String, WireError> {
    let mut bytes = bs58::decode(address)
        .into_vec()
        .map_err(|e| WireError::Tree(e.to_string()))?;
    if bytes.len() <= CHECKSUM_LEN {
        return Err(WireError::Tree("address too short".into()));
    }
    let body = bytes.len() - CHECKSUM_LEN;
    if bytes[0] & 0xF0 != from {
        return Err(WireError::Tree("address is for another network".into()));
    }
    if bytes[body..] != blake2b256(&bytes[..body])[..CHECKSUM_LEN] {
        return Err(WireError::Tree("bad address checksum".into()));
    }
    bytes[0] = (bytes[0] & 0x0F) | to;
    let checksum = blake2b256(&bytes[..body]);
    bytes[body..].copy_from_slice(&checksum[..CHECKSUM_LEN]);
    Ok(bs58::encode(bytes).into_string())
}

/// Address string for `prefix`. ergo-lib knows only prefixes 0 and 16; any other prefix is
/// produced by re-prefixing the mainnet string.
pub fn encode_address(prefix: u8, addr: &Address) -> String {
    match prefix {
        0 => AddressEncoder::new(NetworkPrefix::Mainnet).address_to_str(addr),
        16 => AddressEncoder::new(NetworkPrefix::Testnet).address_to_str(addr),
        other => {
            let mainnet = AddressEncoder::new(NetworkPrefix::Mainnet).address_to_str(addr);
            // A string ergo-lib just produced always has prefix 0 and a valid checksum.
            reprefix(&mainnet, 0, other).expect("re-prefixing a freshly encoded address")
        }
    }
}

/// Parse an address that must carry `prefix`.
///
/// ergo-lib's testnet parser accepts any header byte of 16 or more, so prefixes 32, 48 and
/// so on would pass as testnet. Every non-zero prefix therefore goes through [`reprefix`],
/// which checks the network nibble exactly.
pub fn parse_address(prefix: u8, address: &str) -> Result<Address, WireError> {
    let parse = |s: &str| {
        AddressEncoder::new(NetworkPrefix::Mainnet)
            .parse_address_from_str(s)
            .map_err(|e| WireError::Tree(e.to_string()))
    };
    match prefix {
        0 => parse(address),
        other => parse(&reprefix(address, other, 0)?),
    }
}

/// Validate the supported network without requiring this tree to have appeared on chain.
pub fn address_tree_hash(address: &str) -> Result<TreeHash, WireError> {
    let addr = parse_address(xp_types::chain::get().address_prefix, address)?;
    let tree = addr.script().map_err(|e| WireError::Tree(e.to_string()))?;
    let bytes = tree
        .sigma_serialize_bytes()
        .map_err(|e| WireError::Tree(e.to_string()))?;
    Ok(tree_hash(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    // P2PK tree for the secp256k1 generator point.
    const P2PK_TREE: &str =
        "0008cd0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    fn p2pk() -> Address {
        let tree = ErgoTree::sigma_parse_bytes(&hex::decode(P2PK_TREE).unwrap()).unwrap();
        Address::recreate_from_ergo_tree(&tree).unwrap()
    }

    fn tree_of(addr: &Address) -> Vec<u8> {
        addr.script().unwrap().sigma_serialize_bytes().unwrap()
    }

    #[test]
    fn known_prefixes_match_ergo_lib() {
        let addr = p2pk();
        assert_eq!(
            encode_address(0, &addr),
            AddressEncoder::new(NetworkPrefix::Mainnet).address_to_str(&addr)
        );
        assert_eq!(
            encode_address(16, &addr),
            AddressEncoder::new(NetworkPrefix::Testnet).address_to_str(&addr)
        );
    }

    #[test]
    fn reprefix_agrees_with_ergo_lib_on_testnet() {
        // The generic path, pointed at a prefix ergo-lib does know, must give its answer.
        let addr = p2pk();
        let mainnet = encode_address(0, &addr);
        let testnet = encode_address(16, &addr);
        assert_eq!(reprefix(&mainnet, 0, 16).unwrap(), testnet);
        assert_eq!(reprefix(&testnet, 16, 0).unwrap(), mainnet);
    }

    #[test]
    fn unknown_prefix_round_trips() {
        let addr = p2pk();
        let s = encode_address(32, &addr);
        assert_ne!(s, encode_address(0, &addr));
        assert_ne!(s, encode_address(16, &addr));
        let header = bs58::decode(&s).into_vec().unwrap()[0];
        assert_eq!(header, 32 + 1, "prefix 32 plus the P2PK address type");
        let back = parse_address(32, &s).unwrap();
        assert_eq!(tree_of(&back), hex::decode(P2PK_TREE).unwrap());
    }

    #[test]
    fn wrong_network_and_bad_checksum_are_rejected() {
        let addr = p2pk();
        assert!(parse_address(32, &encode_address(0, &addr)).is_err());
        assert!(parse_address(32, &encode_address(16, &addr)).is_err());
        assert!(parse_address(0, &encode_address(32, &addr)).is_err());
        assert!(parse_address(16, &encode_address(0, &addr)).is_err());
        assert!(parse_address(16, &encode_address(32, &addr)).is_err());
        assert!(parse_address(16, &encode_address(48, &addr)).is_err());
        assert!(parse_address(16, &encode_address(240, &addr)).is_err());
        assert!(parse_address(16, &encode_address(16, &addr)).is_ok());

        let mut bytes = bs58::decode(encode_address(32, &addr)).into_vec().unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        assert!(parse_address(32, &bs58::encode(bytes).into_string()).is_err());
        assert!(parse_address(32, "abc").is_err());
        assert!(parse_address(32, "not base58 0OIl").is_err());
    }
}
