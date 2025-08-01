// test.rs
#![cfg(test)]

use rand::{Fill};

use soroban_sdk::{
    testutils::{Address as _, Ledger}, token, Address, Bytes, Env
};

use crate::{
    AmountCalc, DutchAuction, EscrowClient, EscrowDirection, EscrowError, EscrowFactory,
    EscrowFactoryClient, EscrowImmutables, EscrowState, TimeLocks,
};

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (token::StellarAssetClient<'a>, token::TokenClient<'a>) {
    let address = e.register_stellar_asset_contract_v2(admin.clone()).address();
    (token::StellarAssetClient::new(e, &address), token::TokenClient::new(e, &address))
}

fn create_escrow_factory_contract<'a>(e: &Env) -> EscrowFactoryClient<'a> {
    let address = e.register(EscrowFactory, ());
    EscrowFactoryClient::new(e, &address)
}

// fn generate_hashlock(e: &Env) -> BytesN<32> {
//     let mut arr = [0u8; 32];
//     e.prng().fill(&mut arr);
//     BytesN::from_array(e, &arr)
// }

fn generate_secret(e: &Env) -> Bytes {
    let mut arr = [0u8; 32];
    arr.fill(&mut rand::rng());
    Bytes::from_slice(e, &arr)
}

fn jump_time(e: &Env, gap: u64) {
    e.ledger().set_timestamp(e.ledger().timestamp() + gap);
}

#[test]
fn test_create_escrow_maker_to_taker_flat_amount() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Check initial state
    assert_eq!(escrow.get_state(), EscrowState::Active);
    assert_eq!(escrow.get_immutables(), immutables);

    let resolves = escrow.get_resolves();
    assert_eq!(resolves.taker, taker);
    assert_eq!(resolves.amount, 500);
    assert_eq!(resolves.timestamp, e.ledger().timestamp());

    // Check token balances
    assert_eq!(token.balance(&maker), 500); // 1000 - 500
    assert_eq!(token.balance(&escrow_address), 500);
    assert_eq!(safety_token.balance(&taker), 50); // 100 - 50
    assert_eq!(safety_token.balance(&escrow_address), 50);
}

#[test]
fn test_create_escrow_taker_to_maker_linear_amount() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&taker, &1000);
    _safety_token.mint(&taker, &100);

    let current_time = e.ledger().timestamp();
    let dutch_auction = DutchAuction {
        start_time: current_time,
        end_time: current_time + 1000,
        start_amount: 500,
        end_amount: 300,
    };

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Taker2Maker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Linear(dutch_auction),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Check initial state
    assert_eq!(escrow.get_state(), EscrowState::Active);

    let resolves = escrow.get_resolves();
    assert_eq!(resolves.taker, taker);
    assert_eq!(resolves.amount, 500); // At start_time, should be start_amount
    assert_eq!(resolves.timestamp, e.ledger().timestamp());

    // Check token balances
    assert_eq!(token.balance(&taker), 500); // 1000 - 500
    assert_eq!(token.balance(&escrow_address), 500);
    assert_eq!(safety_token.balance(&taker), 50); // 100 - 50
    assert_eq!(safety_token.balance(&escrow_address), 50);
}

#[test]
fn test_withdraw_with_correct_secret() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Advance time past withdrawal timelock
    jump_time(&e, 1001);

    // Withdraw with correct secret
    escrow.withdraw(&secret, &taker);

    // Check final state
    assert_eq!(escrow.get_state(), EscrowState::Withdrawn);

    // Check token balances
    assert_eq!(token.balance(&maker), 500);
    assert_eq!(token.balance(&taker), 500);
    assert_eq!(safety_token.balance(&taker), 100); // 50 + 50 safety deposit
    assert_eq!(safety_token.balance(&escrow_address), 0);
}

#[test]
fn test_withdraw_with_incorrect_secret() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);
    let wrong_secret = generate_secret(&e);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Advance time past withdrawal timelock
    jump_time(&e, 1001);

    // Try to withdraw with wrong secret
    let error = escrow.try_withdraw(&wrong_secret, &taker);
    assert_eq!(error.err(), Some(Ok(EscrowError::InvalidSecret.into())));

    // State should remain active
    assert_eq!(escrow.get_state(), EscrowState::Active);
}

#[test]
fn test_withdraw_too_early() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Try to withdraw before timelock
    let error = escrow.try_withdraw(&secret, &taker);
    assert_eq!(error.err(), Some(Ok(EscrowError::TooEarly.into())));

    // State should remain active
    assert_eq!(escrow.get_state(), EscrowState::Active);
}

#[test]
fn test_cancel_by_taker() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Advance time past cancellation timelock
    jump_time(&e, 3001);

    // Cancel by taker
    escrow.cancel(&taker);

    // Check final state
    assert_eq!(escrow.get_state(), EscrowState::Cancelled);

    // Check token balances
    assert_eq!(token.balance(&maker), 1000); // Full amount returned
    assert_eq!(token.balance(&escrow_address), 0);
    assert_eq!(safety_token.balance(&taker), 100); // 50 + 50 safety deposit
    assert_eq!(safety_token.balance(&escrow_address), 0);
}

#[test]
fn test_cancel_by_public_too_early() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);
    let public = Address::generate(&e);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Try to cancel by public before timelock
    let error = escrow.try_cancel(&public);
    assert_eq!(error.err(), Some(Ok(EscrowError::TooEarly.into())));

    // State should remain active
    assert_eq!(escrow.get_state(), EscrowState::Active);
}

#[test]
fn test_double_withdraw() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Advance time past withdrawal timelock
    jump_time(&e, 1001);

    // First withdrawal
    escrow.withdraw(&secret, &taker);

    // Try to withdraw again
    let error = escrow.try_withdraw(&secret, &taker);
    assert_eq!(error.err(), Some(Ok(EscrowError::NotActive.into())));
}

#[test]
fn test_withdraw_after_cancel() {
    let e = Env::default();
    e.mock_all_auths();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    // Mint tokens
    _token.mint(&maker, &1000);
    _safety_token.mint(&taker, &100);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    let escrow_address = factory.create_escrow(&immutables, &taker);
    let escrow = EscrowClient::new(&e, &escrow_address);

    // Advance time past cancellation timelock
    jump_time(&e, 3001);

    // Cancel
    escrow.cancel(&taker);

    // Try to withdraw after cancel
    let error = escrow.try_withdraw(&secret, &taker);
    assert_eq!(error.err(), Some(Ok(EscrowError::NotActive.into())));
}

#[test]
fn test_dutch_auction_amount_calculation() {
    let start_time = 1000;
    let end_time = 2000;
    let dutch_auction = DutchAuction {
        start_time,
        end_time,
        start_amount: 1000,
        end_amount: 500,
    };

    let calc = AmountCalc::Linear(dutch_auction);

    // At start time
    assert_eq!(calc.calc(start_time), 1000);

    // At end time
    assert_eq!(calc.calc(end_time), 500);

    // Midpoint
    assert_eq!(calc.calc(1500), 750);

    // Before start (clamped)
    assert_eq!(calc.calc(500), 1000);

    // After end (clamped)
    assert_eq!(calc.calc(2500), 500);
}

#[test]
fn test_create_escrow_unauthorized_taker() {
    let e = Env::default();

    let factory = create_escrow_factory_contract(&e);
    let token_admin = Address::generate(&e);
    let (_token, token) = create_token_contract(&e, &token_admin);
    let (_safety_token, safety_token) = create_token_contract(&e, &token_admin);

    let maker = Address::generate(&e);
    let unauthorized_taker = Address::generate(&e);
    let secret = generate_secret(&e);
    let hashlock = e.crypto().sha256(&secret);

    let immutables = EscrowImmutables {
        hashlock: hashlock.to_bytes(),
        direction: EscrowDirection::Maker2Taker,
        maker: maker.clone(),
        token: token.address.clone(),
        amount: AmountCalc::Flat(500),
        safety_deposit_token: safety_token.address.clone(),
        safety_deposit_amount: 50,
        timelocks: TimeLocks {
            withdrawal: 1000,
            public_withdrawal: 2000,
            cancellation: 3000,
            public_cancellation: 4000,
        },
    };

    // Try to create escrow with unauthorized taker
    let error = factory.try_create_escrow(&immutables, &unauthorized_taker);
    assert!(error.is_err());
}