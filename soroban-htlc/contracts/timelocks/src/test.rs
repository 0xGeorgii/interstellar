#![cfg(test)]

use super::*;
use soroban_sdk::{Env};

#[test]
fn test_timelocks() {
    let env = Env::default();

    let deployed_at = 1_000_000u32;
    let mut timelocks = Timelocks::new(&env, deployed_at);

    timelocks.set_stage(Stage::SrcWithdrawal, 300);
    timelocks.set_stage(Stage::DstWithdrawal, 600);

    assert_eq!(timelocks.get(Stage::SrcWithdrawal), 1_000_300);
    assert_eq!(timelocks.get(Stage::DstWithdrawal), 1_000_600);
    assert_eq!(timelocks.rescue_start(1000), 1_001_000);
}