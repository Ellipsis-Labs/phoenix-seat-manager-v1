use std::mem::size_of;

use ellipsis_client::program_test::*;
use phoenix::program::*;
use phoenix_seat_manager::instruction_builders::create_name_market_authority_successor_instruction;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;

mod setup;
use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;

#[tokio::test]
async fn test_name_market_authority_successor() {
    let PhoenixTestClient { ctx: _, sdk, .. } = bootstrap_default(0).await;

    // Create seat manager instruction to name market authority successor
    let successor = Pubkey::new_unique();

    let name_market_authority_successor_ix = create_name_market_authority_successor_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        &successor,
    );

    // Send instruction to name market authority successor
    sdk.client
        .sign_send_instructions(
            vec![name_market_authority_successor_ix],
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

    assert_eq!(header.successor, successor);
}

#[tokio::test]
async fn test_name_market_authority_unauthorized_seat_authority_signed() {
    // Same setup as test_name_market_authority_successor except authority is not the payer not a random pubkey
    let PhoenixTestClient { ctx: _, sdk, .. } = bootstrap_default(0).await;

    // Create seat manager instruction to name market authority successor
    let successor = Pubkey::new_unique();
    let incorrect_authority = Keypair::new();

    let name_market_authority_successor_ix = create_name_market_authority_successor_instruction(
        &sdk.active_market_key,
        &incorrect_authority.pubkey(),
        &successor,
    );

    // Send instruction to name market authority successor
    let result = sdk
        .client
        .sign_send_instructions(
            vec![name_market_authority_successor_ix],
            vec![&incorrect_authority],
        )
        .await;

    assert!(result.is_err());
}
