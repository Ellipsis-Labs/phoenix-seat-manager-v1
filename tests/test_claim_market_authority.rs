use std::mem::size_of;

use crate::setup::helpers::airdrop;
use bytemuck::Zeroable;
use phoenix::program::create_change_seat_status_instruction;
use phoenix::program::create_claim_authority_instruction;
use phoenix::program::create_name_successor_instruction;
use phoenix::program::create_request_seat_authorized_instruction;
use phoenix::program::load_with_dispatch;
use phoenix::program::status::SeatApprovalStatus;
use phoenix::program::MarketHeader;
use phoenix_seat_manager::get_seat_deposit_collector_address;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_add_dmm_instruction;
use phoenix_seat_manager::instruction_builders::create_claim_market_authority_instruction;
use phoenix_seat_manager::instruction_builders::create_claim_seat_authorized_instruction;
use phoenix_seat_manager::instruction_builders::create_name_market_authority_successor_instruction;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use crate::setup::init::PhoenixTestClient;
use crate::setup::init::{bootstrap_default, bootstrap_default_without_sm_claiming_authority};
mod setup;

#[tokio::test]
async fn test_claim_market_authority_reclaim_works_with_fresh_dmms() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    // Add DMM
    let trader_keypair = Keypair::new();

    let claim_seat = create_claim_seat_authorized_instruction(
        &trader_keypair.pubkey(),
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![claim_seat], vec![])
        .await
        .unwrap();

    let add_mm_ix = create_add_dmm_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        &trader_keypair.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![add_mm_ix], vec![])
        .await
        .unwrap();

    let seat_manager_address = get_seat_manager_address(&sdk.active_market_key).0;
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<phoenix_seat_manager::seat_manager::SeatManager>(
        &seat_manager_data,
    )
    .unwrap();

    assert_eq!(
        seat_manager.designated_market_makers[0],
        trader_keypair.pubkey()
    );

    // Pass market authority to new keypair
    let new_authority = Keypair::new();
    let name_market_authority_successor_ix = create_name_market_authority_successor_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        &new_authority.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![name_market_authority_successor_ix], vec![])
        .await
        .unwrap();

    let claim_market_authority =
        create_claim_authority_instruction(&new_authority.pubkey(), &sdk.active_market_key);

    sdk.client
        .sign_send_instructions(vec![claim_market_authority], vec![&new_authority])
        .await
        .unwrap();

    let market_data = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();

    let header_bytes = &market_data[..size_of::<MarketHeader>()];
    let market_header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();
    assert!(market_header.authority == new_authority.pubkey());

    // New authority names seat manager as new market successor
    let name_market_authority_successor_ix = create_name_successor_instruction(
        &new_authority.pubkey(),
        &sdk.active_market_key,
        &seat_manager_address,
    );

    sdk.client
        .sign_send_instructions(
            vec![name_market_authority_successor_ix],
            vec![&new_authority],
        )
        .await
        .unwrap();

    // Seat manager claim authority still works
    let claim_market_authority = create_claim_market_authority_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );
    // Add lamports to the seat collector address because a trader exists on the market.
    let required_lamports = sdk.client.rent_exempt(spl_token::state::Account::LEN) * 2;
    let seat_deposit_collector = get_seat_deposit_collector_address(&sdk.active_market_key).0;

    airdrop(&sdk.client, &seat_deposit_collector, required_lamports)
        .await
        .unwrap();

    sdk.client
        .sign_send_instructions(vec![claim_market_authority], vec![])
        .await
        .unwrap();

    let market_data = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();

    let header_bytes = &market_data[..size_of::<MarketHeader>()];
    let market_header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();
    assert!(market_header.authority == seat_manager_address);

    // Check that DMM was removed
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<phoenix_seat_manager::seat_manager::SeatManager>(
        &seat_manager_data,
    )
    .unwrap();

    assert_eq!(seat_manager.designated_market_makers[0], Pubkey::zeroed());
}

#[tokio::test]
async fn test_claim_market_authority_requires_rent_for_existing_traders() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default_without_sm_claiming_authority(5).await;

    // Add two traders to the market
    let trader_keypair = Keypair::new();
    let trader_keypair_two = Keypair::new();

    let claim_seat = create_request_seat_authorized_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair.pubkey(),
    );

    let claim_seat_two = create_request_seat_authorized_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair_two.pubkey(),
    );

    let change_seat_status_one = create_change_seat_status_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair.pubkey(),
        SeatApprovalStatus::Approved,
    );

    let change_seat_status_two = create_change_seat_status_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair_two.pubkey(),
        SeatApprovalStatus::Approved,
    );

    sdk.client
        .sign_send_instructions(
            vec![
                claim_seat,
                claim_seat_two,
                change_seat_status_one,
                change_seat_status_two,
            ],
            vec![],
        )
        .await
        .unwrap();

    let claim_market_authority = create_claim_market_authority_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );
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

    let required_lamports = sdk.client.rent_exempt(spl_token::state::Account::LEN)
        * 2
        * registered_traders.len() as u64; // Times 2 for two token accounts for each trader

    let seat_deposit_collector = get_seat_deposit_collector_address(&sdk.active_market_key).0;
    println!("Required lamports: {}", required_lamports);

    airdrop(&sdk.client, &seat_deposit_collector, required_lamports)
        .await
        .unwrap();

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_market_authority.clone()], vec![])
        .await
        .is_ok());
}

#[tokio::test]
async fn test_claim_market_authority_fails_if_deposit_collector_has_insufficient_lamports() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default_without_sm_claiming_authority(5).await;

    // Add two traders to the market
    let trader_keypair = Keypair::new();
    let trader_keypair_two = Keypair::new();

    let claim_seat = create_request_seat_authorized_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair.pubkey(),
    );

    let claim_seat_two = create_request_seat_authorized_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair_two.pubkey(),
    );

    let change_seat_status_one = create_change_seat_status_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair.pubkey(),
        SeatApprovalStatus::Approved,
    );

    let change_seat_status_two = create_change_seat_status_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &trader_keypair_two.pubkey(),
        SeatApprovalStatus::Approved,
    );

    sdk.client
        .sign_send_instructions(
            vec![
                claim_seat,
                claim_seat_two,
                change_seat_status_one,
                change_seat_status_two,
            ],
            vec![],
        )
        .await
        .unwrap();

    let claim_market_authority = create_claim_market_authority_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );

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

    let required_lamports = sdk.client.rent_exempt(spl_token::state::Account::LEN)
        * 2
        * registered_traders.len() as u64; // Times two for each registered trader and 2 for two token accounts for each trader

    println!("Required lamports: {}", required_lamports);

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_market_authority.clone()], vec![])
        .await
        .is_err());
}
