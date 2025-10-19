use super::*;
use getrandom::getrandom;

fn matches_suffix_mod_bytes_naive(
    public_key: &[u8; SECRET_LEN],
    params: &SuffixParams,
) -> bool {
    if params.modulus == 1 {
        return true;
    }

    let mut remainder = 0u128;
    for &byte in public_key.iter() {
        remainder = ((remainder << 8) + byte as u128) % params.modulus as u128;
    }
    remainder == params.value as u128
}

#[test]
fn test_suffix_canonicalization() {
    let miner = VanityMiner::new("feel".to_string());
    assert_eq!(miner.get_suffix(), "FEEL");
    assert!(miner.suffix_params.is_some());
}

#[test]
fn test_suffix_case_sensitive_matching() {
    let miner = VanityMiner::new("FEEL".to_string());
    let suffix = miner.suffix_bytes.clone();
    assert!(suffix_matches_exact(b"XYZFEEL", &suffix));
    assert!(!suffix_matches_exact(b"XYZFeel", &suffix));
    assert!(!suffix_matches_exact(b"XYZfeel", &suffix));
    assert!(!suffix_matches_exact(b"XYZFEE1", &suffix));
}

#[test]
fn test_suffix_mod_fast_path_equivalence() {
    let miner = VanityMiner::new("FEEL".to_string());
    let params = miner.suffix_params.expect("suffix params missing");
    for _ in 0..10 {
        let mut public_key = [0u8; SECRET_LEN];
        getrandom(&mut public_key).unwrap();
        let fast = matches_suffix_mod_bytes(&public_key, &params);
        let slow = matches_suffix_mod_bytes_naive(&public_key, &params);
        assert_eq!(fast, slow);
    }
}

#[test]
fn test_suffix_params_fallback_on_long_suffix() {
    let long_suffix = "123456789ABCDEFGHJKLMNPQRSTUV";
    assert!(
        compute_suffix_params(long_suffix.as_bytes()).is_none(),
        "long suffix should fall back to full encode"
    );
}
