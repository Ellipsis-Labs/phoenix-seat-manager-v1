mod setup;
use std::mem::size_of;

use crate::setup::helpers::airdrop;
use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;
use phoenix::program::status::MarketStatus;
use phoenix::program::MarketHeader;
use phoenix_seat_manager::instruction_builders::create_change_market_status_instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn test_change_market_status_happy_path() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let change_market_status = create_change_market_status_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        MarketStatus::Paused,
    );

    sdk.client
        .sign_send_instructions(vec![change_market_status], vec![&sdk.client.payer])
        .await
        .unwrap();

    let market_data = sdk
        .client
        .get_account_data(&sdk.active_market_key)
        .await
        .unwrap();

    let (header_bytes, _) = market_data.split_at(size_of::<MarketHeader>());
    let header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).unwrap();

    assert_eq!(header.status, MarketStatus::Paused as u64);
}

#[tokio::test]
async fn test_change_market_status_fails_if_authority_not_signer_or_unauthorized_signer() {
    let PhoenixTestClient {
        ctx: _,
        mut sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let unauthorized = Keypair::new();
    airdrop(&sdk.client, &unauthorized.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let change_market_status = create_change_market_status_instruction(
        &sdk.active_market_key,
        &unauthorized.pubkey(),
        MarketStatus::Paused,
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![change_market_status], vec![&unauthorized])
        .await
        .is_err());

    let mut change_market_status = create_change_market_status_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        MarketStatus::Paused,
    );

    change_market_status.accounts[4].is_signer = false;
    sdk.client.add_keypair(&unauthorized);
    sdk.client.set_payer(&unauthorized.pubkey()).unwrap();

    assert!(sdk
        .client
        .sign_send_instructions(vec![change_market_status], vec![])
        .await
        .is_err());
}
