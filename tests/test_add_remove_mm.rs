mod setup;

use crate::setup::helpers::airdrop;
use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_add_dmm_instruction;
use phoenix_seat_manager::instruction_builders::create_claim_seat_authorized_instruction;
use phoenix_seat_manager::instruction_builders::create_remove_dmm_instruction;
use phoenix_seat_manager::seat_manager::SeatManager;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn test_add_remove_happy_path() {
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

    // Remove designated MM
    let remove_as_mm =
        create_remove_dmm_instruction(&sdk.active_market_key, &sdk.client.payer.pubkey(), &trader);
    sdk.client
        .sign_send_instructions(vec![remove_as_mm], vec![])
        .await
        .unwrap();

    let (seat_manager_address, _) = get_seat_manager_address(&sdk.active_market_key);
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data).unwrap();
    assert_eq!(seat_manager.num_makers, 0);
}

#[tokio::test]
async fn test_add_remove_fails_if_trader_has_no_seat() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let trader = Pubkey::new_unique();

    // Add designated MM errors if no seat
    let add_as_mm =
        create_add_dmm_instruction(&sdk.active_market_key, &sdk.client.payer.pubkey(), &trader);

    assert!(sdk
        .client
        .sign_send_instructions(vec![add_as_mm], vec![])
        .await
        .is_err());
    let remove_as_mm =
        create_remove_dmm_instruction(&sdk.active_market_key, &sdk.client.payer.pubkey(), &trader);

    assert!(sdk
        .client
        .sign_send_instructions(vec![remove_as_mm], vec![])
        .await
        .is_err());
}

#[tokio::test]
async fn test_add_remove_fails_if_authority_not_signer_or_authority_mismatch() {
    let PhoenixTestClient {
        ctx: _,
        mut sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

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

    let unauthorized = Keypair::new();
    airdrop(&sdk.client, &unauthorized.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let add_as_mm =
        create_add_dmm_instruction(&sdk.active_market_key, &unauthorized.pubkey(), &trader);

    // Signer that does not match seat manager authority (client.payer) fails
    assert!(sdk
        .client
        .sign_send_instructions(vec![add_as_mm.clone()], vec![&unauthorized])
        .await
        .is_err());

    // Missing authority signer fails
    let mut add_as_mm =
        create_add_dmm_instruction(&sdk.active_market_key, &sdk.client.payer.pubkey(), &trader);

    add_as_mm.accounts[3].is_signer = false;
    sdk.client.add_keypair(&unauthorized);
    sdk.client.set_payer(&unauthorized.pubkey()).unwrap();

    assert!(sdk
        .client
        .sign_send_instructions(vec![add_as_mm], vec![])
        .await
        .is_err());
}
