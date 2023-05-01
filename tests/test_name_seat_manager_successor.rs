mod setup;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_name_seat_manager_successor_instruction;
use phoenix_seat_manager::seat_manager::SeatManager;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use crate::setup::helpers::airdrop;
use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;

#[tokio::test]
async fn test_name_seat_manager_successor_happy_path() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let successor = Pubkey::new_unique();

    let name_successor_ix = create_name_seat_manager_successor_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &successor,
    );

    sdk.client
        .sign_send_instructions(vec![name_successor_ix], vec![])
        .await
        .unwrap();

    let (seat_manager_address, _) = get_seat_manager_address(&sdk.active_market_key);
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data).unwrap();

    assert_eq!(seat_manager.successor, successor);
}

#[tokio::test]
async fn test_name_seat_manager_sucesssor_fails_if_authority_not_signer_or_authority_mismatch() {
    let PhoenixTestClient {
        ctx: _,
        mut sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let successor = Keypair::new().pubkey();
    let unauthorized = Keypair::new();
    airdrop(&sdk.client, &unauthorized.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Fails if authority mismatch with seat manager account (real authority is sdk.client.pubkey)
    let name_successor_ix = create_name_seat_manager_successor_instruction(
        &unauthorized.pubkey(),
        &sdk.active_market_key,
        &successor,
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![name_successor_ix], vec![&unauthorized])
        .await
        .is_err());

    // Fails if authority is not a signer
    let mut name_successor_ix = create_name_seat_manager_successor_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
        &successor,
    );

    name_successor_ix.accounts[1].is_signer = false;
    sdk.client.add_keypair(&unauthorized);
    sdk.client.set_payer(&unauthorized.pubkey()).unwrap();

    assert!(sdk
        .client
        .sign_send_instructions(vec![name_successor_ix], vec![])
        .await
        .is_err());
}
