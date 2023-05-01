mod setup;
use std::mem::size_of;

use phoenix::program::create_new_order_instruction;
use phoenix::program::load_with_dispatch;
use phoenix::program::MarketHeader;
use phoenix::quantities::WrapperU64;
use phoenix::state::OrderPacket;
use phoenix::state::SelfTradeBehavior;
use phoenix::state::Side;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_change_market_fee_recipient_instruction;
use solana_program::pubkey::Pubkey;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use crate::setup::helpers::{get_and_bootstrap_maker, get_and_bootstrap_taker};
use crate::setup::init::*;

#[tokio::test]
async fn test_change_market_fee_recipient_no_unclaimed() {
    let PhoenixTestClient { ctx: _, sdk, .. } = bootstrap_default(0).await;
    println!("Seat program authority: {}", sdk.client.payer.pubkey());

    let new_fee_recipient = Pubkey::new_unique();

    let change_market_fee_recipient_ix = create_change_market_fee_recipient_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        &new_fee_recipient,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
    );

    let market_account_data = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();
    let (header_bytes, _market_bytes) = market_account_data.split_at(size_of::<MarketHeader>());
    let header: &MarketHeader = bytemuck::try_from_bytes(header_bytes).unwrap();

    println!("Current market authority: {}", header.authority);

    sdk.client
        .sign_send_instructions(
            vec![change_market_fee_recipient_ix],
            vec![&sdk.client.payer],
        )
        .await
        .unwrap();
    // Verify that the market authority successor is set to the new keypair
    let market_account_data = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();
    let (header_bytes, _bytes) = market_account_data.split_at(size_of::<MarketHeader>());

    let header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();

    assert_eq!(header.fee_recipient, new_fee_recipient);
}

#[tokio::test]
async fn test_change_market_fee_recipient_unclaimed_fees() {
    // Create a trade on the market and assert that there are some fees in the market
    let PhoenixTestClient {
        ctx: _,
        mut sdk,
        mint_authority,
    } = bootstrap_default(5).await;
    //Place limit order with maker (keypair 1)
    let maker = get_and_bootstrap_maker(&mut sdk, &mint_authority).await;
    let maker_order_packet = OrderPacket::new_limit_order_default(
        Side::Ask,
        sdk.float_price_to_ticks(10.0),
        1_000_000_000,
    );
    let limit_order_ix = create_new_order_instruction(
        &sdk.active_market_key,
        &maker.user.pubkey(),
        &sdk.base_mint,
        &sdk.quote_mint,
        &maker_order_packet,
    );
    sdk.client
        .sign_send_instructions(vec![limit_order_ix], vec![&maker.user])
        .await
        .unwrap();

    //Place cross order with taker (keypair 2)
    let taker = get_and_bootstrap_taker(&mut sdk, &mint_authority).await;
    let taker_order_packet = OrderPacket::new_ioc_buy_with_limit_price(
        sdk.float_price_to_ticks(10.0),
        10_000_000,
        SelfTradeBehavior::Abort,
        None,
        10,
        false,
    );
    let taker_order_ix = create_new_order_instruction(
        &sdk.active_market_key,
        &taker.user.pubkey(),
        &sdk.base_mint,
        &sdk.quote_mint,
        &taker_order_packet,
    );

    sdk.client
        .sign_send_instructions(vec![taker_order_ix], vec![&taker.user])
        .await
        .unwrap();

    // Assert that market has unclaimed fees
    let (unclaimed_fees, current_fee_recipient) = {
        // Check if there are unclaimed fees in the market account. If so, generate change fee with unclaimed ix
        let market_data = sdk
            .client
            .get_account_data(&sdk.active_market_key)
            .await
            .unwrap();
        let (header_bytes, market_bytes) = market_data.split_at(size_of::<MarketHeader>());
        let market_header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();

        println!("Current authority: {}", market_header.authority);

        let seat = get_seat_manager_address(&sdk.active_market_key);
        println!("Seat manager address: {:?}", seat);
        println!("Seat manager id: {}", phoenix_seat_manager::id());
        println!("Current fee recipient: {}", market_header.fee_recipient);
        println!("SDK payer: {}", sdk.client.payer.pubkey());

        let market = load_with_dispatch(&market_header.market_size_params, market_bytes)
            .unwrap()
            .inner;
        (
            market.get_uncollected_fee_amount(),
            market_header.fee_recipient,
        )
    };

    println!("Unclaimed fees: {}", unclaimed_fees.as_u64());

    assert!(unclaimed_fees.as_u64() > 0);

    let new_fee_recipient = Pubkey::new_unique();

    let claim_fees_ix = create_change_market_fee_recipient_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        &new_fee_recipient,
        &sdk.quote_mint,
        &current_fee_recipient,
    );

    sdk.client
        .sign_send_instructions(
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                claim_fees_ix,
            ],
            vec![&sdk.client.payer],
        )
        .await
        .unwrap();

    // Assert that the market authority successor is set to the new keypair
    let market_account_data = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();
    let (header_bytes, _bytes) = market_account_data.split_at(size_of::<MarketHeader>());

    let header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();

    assert_eq!(header.fee_recipient, new_fee_recipient);
}

#[tokio::test]
async fn test_change_market_fee_recipient_invalid_authority() {
    let PhoenixTestClient { ctx: _, sdk, .. } = bootstrap_default(0).await;
    println!("Seat program authority: {}", sdk.client.payer.pubkey());

    let new_fee_recipient = Pubkey::new_unique();
    let incorrect_authority = Keypair::new();

    let change_market_fee_recipient_ix = create_change_market_fee_recipient_instruction(
        &sdk.active_market_key,
        &incorrect_authority.pubkey(),
        &new_fee_recipient,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
    );

    assert!(sdk
        .client
        .sign_send_instructions(
            vec![change_market_fee_recipient_ix],
            vec![&incorrect_authority],
        )
        .await
        .is_err());
}
