mod setup;

use std::mem::size_of;

use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;
use crate::setup::init::{setup_account, NUM_SEATS};
use phoenix::program::load_with_dispatch;
use phoenix::program::MarketHeader;
use phoenix_seat_manager::get_seat_deposit_collector_address;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_add_dmm_instruction;
use phoenix_seat_manager::instruction_builders::create_claim_seat_authorized_instruction;
use phoenix_seat_manager::instruction_builders::EvictTraderAccountBackup;
use phoenix_seat_manager::instruction_builders::{
    create_claim_seat_instruction, create_evict_seat_instruction,
};
use phoenix_seat_manager::seat_manager::SeatManager;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction::burn;
use spl_token::instruction::close_account;

#[tokio::test]
async fn test_evict_seat_multiple_authorized() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    // Claim seats for two traders
    let trader_one =
        setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint).await;

    let trader_two =
        setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint).await;

    let claim_seat_one =
        create_claim_seat_instruction(&trader_one.user.pubkey(), &sdk.active_market_key);

    let claim_seat_two =
        create_claim_seat_instruction(&trader_two.user.pubkey(), &sdk.active_market_key);

    sdk.client
        .sign_send_instructions(
            vec![claim_seat_one, claim_seat_two],
            vec![&trader_one.user, &trader_two.user],
        )
        .await
        .unwrap();

    let traders = sdk.get_traders().await;
    assert!(traders.get(&trader_one.user.pubkey()).is_some());
    assert!(traders.get(&trader_two.user.pubkey()).is_some());

    // Evict seats for both traders
    let evict_seats = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
        vec![
            EvictTraderAccountBackup {
                trader_pubkey: trader_one.user.pubkey(),
                base_token_account_backup: None,
                quote_token_account_backup: None,
            },
            EvictTraderAccountBackup {
                trader_pubkey: trader_two.user.pubkey(),
                base_token_account_backup: None,
                quote_token_account_backup: None,
            },
        ],
    );

    let compute_increase = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    sdk.client
        .sign_send_instructions(vec![compute_increase, evict_seats], vec![])
        .await
        .unwrap();

    // Assert that neither trader are in the market state
    let traders = sdk.get_traders().await;
    assert!(traders.get(&trader_one.user.pubkey()).is_none());
    assert!(traders.get(&trader_two.user.pubkey()).is_none());
}

#[tokio::test]
async fn test_evict_seat_permissionless_succeeds_when_full_and_only_evicts_one() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader_one =
        setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint).await;

    let trader_two =
        setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint).await;

    let claim_seat_one =
        create_claim_seat_instruction(&trader_one.user.pubkey(), &sdk.active_market_key);

    let claim_seat_two =
        create_claim_seat_instruction(&trader_two.user.pubkey(), &sdk.active_market_key);

    sdk.client
        .sign_send_instructions(
            vec![claim_seat_one, claim_seat_two],
            vec![&trader_one.user, &trader_two.user],
        )
        .await
        .unwrap();

    let traders = sdk.get_traders().await;
    assert!(traders.get(&trader_one.user.pubkey()).is_some());
    assert!(traders.get(&trader_two.user.pubkey()).is_some());

    // Need to fill market with traders
    let num_new_traders = NUM_SEATS + 100;
    for _ in 0..num_new_traders {
        let trader = Pubkey::new_unique();

        let claim_seat = create_claim_seat_authorized_instruction(
            &trader,
            &sdk.active_market_key,
            &sdk.client.payer.pubkey(),
        );

        let result = sdk
            .client
            .sign_send_instructions(vec![claim_seat], vec![])
            .await;
        println!("Result: {:?}", result);
    }

    let market_bytes = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();
    let (header_bytes, market_bytes) = market_bytes.split_at(size_of::<MarketHeader>());
    let market_header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();

    let market = load_with_dispatch(&market_header.market_size_params, market_bytes)
        .unwrap()
        .inner;

    let registered_traders = market.get_registered_traders();

    println!("Capacity: {:?}", registered_traders.capacity());
    println!("Length: {:?}", registered_traders.len());

    assert!(registered_traders.capacity() == registered_traders.len());

    // Evict two traders permissionless
    let unauthorized_keypair = Keypair::new();

    let evict_seats = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &unauthorized_keypair.pubkey(),
        vec![
            EvictTraderAccountBackup {
                trader_pubkey: trader_one.user.pubkey(),
                base_token_account_backup: None,
                quote_token_account_backup: None,
            },
            EvictTraderAccountBackup {
                trader_pubkey: trader_two.user.pubkey(),
                base_token_account_backup: None,
                quote_token_account_backup: None,
            },
        ],
    );

    sdk.client
        .sign_send_instructions(
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                evict_seats,
            ],
            vec![&unauthorized_keypair],
        )
        .await
        .unwrap();

    // Assert that trader one was evicted but trader two remains
    let traders = sdk.get_traders().await;
    assert!(traders.get(&trader_one.user.pubkey()).is_none());
    assert!(traders.get(&trader_two.user.pubkey()).is_some());
}

#[tokio::test]
// This test MUST PASS because remove designated market maker relies on that mm having a seat
async fn test_evict_seat_fails_on_designated_market_maker() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    // Add seat for trader
    let trader = Pubkey::new_unique();

    let claim_seat = create_claim_seat_authorized_instruction(
        &trader,
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![claim_seat], vec![])
        .await
        .unwrap();

    // Add designated MM
    let add_as_mm =
        create_add_dmm_instruction(&sdk.active_market_key, &sdk.client.payer.pubkey(), &trader);

    sdk.client
        .sign_send_instructions(vec![add_as_mm], vec![])
        .await
        .unwrap();

    let (seat_manager_address, _) = get_seat_manager_address(&sdk.active_market_key);
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data).unwrap();

    assert_eq!(seat_manager.num_makers, 1);

    // Evict seats for dmm - should fail
    let evict_seat = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
        vec![EvictTraderAccountBackup {
            trader_pubkey: trader,
            base_token_account_backup: None,
            quote_token_account_backup: None,
        }],
    );

    sdk.client
        .sign_send_instructions(vec![evict_seat], vec![])
        .await
        .unwrap();

    let (seat_manager_address, _) = get_seat_manager_address(&sdk.active_market_key);
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data).unwrap();

    assert_eq!(seat_manager.num_makers, 1);
}

#[tokio::test]
async fn test_evict_seat_refunds_claim_seat_deposit() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader = setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint)
        .await
        .user;
    let claim_seat_ix = create_claim_seat_instruction(&trader.pubkey(), &sdk.active_market_key);

    sdk.client
        .sign_send_instructions(vec![claim_seat_ix], vec![&trader])
        .await
        .unwrap();

    // Evict seat then check that seat_deposit_collector has the same lamports as initial and that trader lamports went up
    let trader_initial_lamports = sdk
        .client
        .get_account(&trader.pubkey())
        .await
        .unwrap()
        .lamports;
    let evict_seat_ix = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
        vec![EvictTraderAccountBackup {
            trader_pubkey: trader.pubkey(),
            base_token_account_backup: None,
            quote_token_account_backup: None,
        }],
    );

    sdk.client
        .sign_send_instructions(
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                evict_seat_ix,
            ],
            vec![],
        )
        .await
        .unwrap();
    let seat_deposit_collector_address =
        get_seat_deposit_collector_address(&sdk.active_market_key).0;

    assert!(sdk
        .client
        .get_account(&seat_deposit_collector_address)
        .await
        .is_err());

    let trader_final_lamports = sdk
        .client
        .get_account(&trader.pubkey())
        .await
        .unwrap()
        .lamports;

    let deposit_amount = sdk.client.rent_exempt(spl_token::state::Account::LEN) * 2;

    assert_eq!(
        trader_final_lamports,
        trader_initial_lamports + deposit_amount
    );
}

#[tokio::test]
async fn test_evict_seat_no_refunds_if_trader_closes_ata() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader = setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint)
        .await
        .user;
    let claim_seat_ix = create_claim_seat_instruction(&trader.pubkey(), &sdk.active_market_key);

    sdk.client
        .sign_send_instructions(vec![claim_seat_ix], vec![&trader])
        .await
        .unwrap();

    // Close trader ata. Must burn all tokens first
    let base_ata = get_associated_token_address(&trader.pubkey(), &sdk.base_mint);
    let base_amount = spl_token::state::Account::unpack_from_slice(
        sdk.client
            .get_account_data(&base_ata)
            .await
            .unwrap()
            .as_slice(),
    )
    .unwrap()
    .amount;
    let base_burn_ix = burn(
        &spl_token::id(),
        &base_ata,
        &sdk.base_mint,
        &trader.pubkey(),
        &[&trader.pubkey()],
        base_amount,
    )
    .unwrap();

    let quote_ata = get_associated_token_address(&trader.pubkey(), &sdk.quote_mint);
    let quote_amount = spl_token::state::Account::unpack_from_slice(
        sdk.client
            .get_account_data(&quote_ata)
            .await
            .unwrap()
            .as_slice(),
    )
    .unwrap()
    .amount;

    let quote_burn_ix = burn(
        &spl_token::id(),
        &quote_ata,
        &sdk.quote_mint,
        &trader.pubkey(),
        &[&trader.pubkey()],
        quote_amount,
    )
    .unwrap();

    sdk.client
        .sign_send_instructions(vec![base_burn_ix, quote_burn_ix], vec![&trader])
        .await
        .unwrap();

    let close_base_ata_ix = close_account(
        &spl_token::id(),
        &base_ata,
        &trader.pubkey(),
        &trader.pubkey(),
        &[&trader.pubkey()],
    )
    .unwrap();

    let close_quote_ata_ix = close_account(
        &spl_token::id(),
        &quote_ata,
        &trader.pubkey(),
        &trader.pubkey(),
        &[&trader.pubkey()],
    )
    .unwrap();

    sdk.client
        .sign_send_instructions(vec![close_base_ata_ix, close_quote_ata_ix], vec![&trader])
        .await
        .unwrap();

    // Evict seat then check that seat_deposit_collector has no lamports (after creating ATAs) and that the trader lamports did not change
    let trader_initial_lamports = sdk
        .client
        .get_account(&trader.pubkey())
        .await
        .unwrap()
        .lamports;

    let evict_seat_ix = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
        vec![EvictTraderAccountBackup {
            trader_pubkey: trader.pubkey(),
            base_token_account_backup: None,
            quote_token_account_backup: None,
        }],
    );

    sdk.client
        .sign_send_instructions(
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                evict_seat_ix,
            ],
            vec![],
        )
        .await
        .unwrap();

    // Seat deposit collector should have no more lamports, due to using deposit to create ATAs
    let seat_deposit_collector_address =
        get_seat_deposit_collector_address(&sdk.active_market_key).0;
    assert!(sdk
        .client
        .get_account(&seat_deposit_collector_address)
        .await
        .is_err());

    // Trader should have the same lamports as before, due to closing of ATAs
    let trader_final_lamports = sdk
        .client
        .get_account(&trader.pubkey())
        .await
        .unwrap()
        .lamports;
    assert_eq!(trader_final_lamports, trader_initial_lamports);
}

#[tokio::test]
async fn test_evict_seat_change_ata_owners_uses_backup_token_accounts() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader = setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint)
        .await
        .user;
    let claim_seat_ix = create_claim_seat_instruction(&trader.pubkey(), &sdk.active_market_key);

    sdk.client
        .sign_send_instructions(vec![claim_seat_ix], vec![&trader])
        .await
        .unwrap();

    // Change ATA owners
    let new_owner = Pubkey::new_unique();
    let base_ata = get_associated_token_address(&trader.pubkey(), &sdk.base_mint);
    let change_owner = spl_token::instruction::set_authority(
        &spl_token::id(),
        &base_ata,
        Some(&new_owner),
        spl_token::instruction::AuthorityType::AccountOwner,
        &trader.pubkey(),
        &[&trader.pubkey()],
    )
    .unwrap();

    let quote_ata = get_associated_token_address(&trader.pubkey(), &sdk.quote_mint);
    let quote_change_owner = spl_token::instruction::set_authority(
        &spl_token::id(),
        &quote_ata,
        Some(&new_owner),
        spl_token::instruction::AuthorityType::AccountOwner,
        &trader.pubkey(),
        &[&trader.pubkey()],
    )
    .unwrap();

    sdk.client
        .sign_send_instructions(vec![change_owner, quote_change_owner], vec![&trader])
        .await
        .unwrap();

    // Create backup token accounts for the trader
    let new_base_token_account = Keypair::new();
    let create_base_account = solana_program::system_instruction::create_account(
        &sdk.client.payer.pubkey(),
        &new_base_token_account.pubkey(),
        Rent::default().minimum_balance(165),
        165,
        &spl_token::id(),
    );

    let create_backup_base_token = spl_token::instruction::initialize_account3(
        &spl_token::id(),
        &new_base_token_account.pubkey(),
        &sdk.base_mint,
        &trader.pubkey(),
    )
    .unwrap();
    let new_quote_token_account = Keypair::new();
    let create_quote_account = solana_program::system_instruction::create_account(
        &sdk.client.payer.pubkey(),
        &new_quote_token_account.pubkey(),
        Rent::default().minimum_balance(165),
        165,
        &spl_token::id(),
    );
    let create_backup_quote_token = spl_token::instruction::initialize_account3(
        &spl_token::id(),
        &new_quote_token_account.pubkey(),
        &sdk.quote_mint,
        &trader.pubkey(),
    );

    println!("Creating new token accounts");
    sdk.client
        .sign_send_instructions(
            vec![create_base_account, create_quote_account],
            vec![&new_base_token_account, &new_quote_token_account],
        )
        .await
        .unwrap();

    println!("Initializing new token accounts");
    sdk.client
        .sign_send_instructions(
            vec![create_backup_base_token, create_backup_quote_token.unwrap()],
            vec![],
        )
        .await
        .unwrap();
    println!("Done initializing token accounts");

    let trader_initial_lamports = sdk
        .client
        .get_account(&trader.pubkey())
        .await
        .unwrap()
        .lamports;

    let signer_initial_lamports = sdk
        .client
        .get_account(&sdk.client.payer.pubkey())
        .await
        .unwrap()
        .lamports;

    // Evict seat with backup token accounts
    let evict_seat_ix = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
        vec![EvictTraderAccountBackup {
            trader_pubkey: trader.pubkey(),
            base_token_account_backup: Some(new_base_token_account.pubkey()),
            quote_token_account_backup: Some(new_quote_token_account.pubkey()),
        }],
    );

    sdk.client
        .sign_send_instructions(
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                evict_seat_ix,
            ],
            vec![],
        )
        .await
        .unwrap();

    let trader_final_lamports = sdk
        .client
        .get_account(&trader.pubkey())
        .await
        .unwrap()
        .lamports;

    let signer_final_lamports = sdk
        .client
        .get_account(&sdk.client.payer.pubkey())
        .await
        .unwrap()
        .lamports;

    let seat_deposit_collector_address =
        get_seat_deposit_collector_address(&sdk.active_market_key).0;

    // Seat deposit collector account should be empty after refunding signer for creation of backup accounts
    assert!(sdk
        .client
        .get_account(&seat_deposit_collector_address)
        .await
        .is_err());

    // Signer gets the lamport refund because signer had to create backup token accounts
    assert!(signer_final_lamports > signer_initial_lamports);
    assert_eq!(trader_final_lamports, trader_initial_lamports);
}
