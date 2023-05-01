mod setup;

use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_claim_seat_manager_authority_instruction;
use phoenix_seat_manager::instruction_builders::create_name_seat_manager_successor_instruction;
use phoenix_seat_manager::seat_manager::SeatManager;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;

#[tokio::test]
async fn test_claim_seat_manager_authority_happy_path() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let successor = Keypair::new();

    let name_successor_ix = create_name_seat_manager_successor_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &successor.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![name_successor_ix], vec![])
        .await
        .unwrap();

    let claim_authority_ix = create_claim_seat_manager_authority_instruction(
        &sdk.active_market_key,
        &successor.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![claim_authority_ix], vec![&successor])
        .await
        .unwrap();

    let (seat_manager_address, _) = get_seat_manager_address(&sdk.active_market_key);
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data).unwrap();

    assert_eq!(seat_manager.authority, successor.pubkey());
}

#[tokio::test]
async fn test_claim_seat_manager_authority_fails_if_sucessor_not_signer_or_wrong_sucessor() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let successor = Keypair::new();

    let name_successor_ix = create_name_seat_manager_successor_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &successor.pubkey(),
    );

    sdk.client
        .sign_send_instructions(vec![name_successor_ix], vec![])
        .await
        .unwrap();

    let claim_authority_ix = create_claim_seat_manager_authority_instruction(
        &sdk.active_market_key,
        &successor.pubkey(),
    );

    // Fails if sucessor is not a signer
    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_authority_ix], vec![])
        .await
        .is_err());

    // Fails if wrong sucessor tries to claim
    let unauthorsized_successor = Keypair::new();

    let claim_authority_ix = create_claim_seat_manager_authority_instruction(
        &sdk.active_market_key,
        &unauthorsized_successor.pubkey(),
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_authority_ix], vec![&unauthorsized_successor])
        .await
        .is_err());
}
