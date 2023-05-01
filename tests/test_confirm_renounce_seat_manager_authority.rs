mod setup;
use phoenix::program::status::MarketStatus;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_change_market_status_instruction;
use phoenix_seat_manager::instruction_builders::create_confirm_renounce_seat_manager_authority_instruction;
use phoenix_seat_manager::instruction_builders::create_initiate_renounce_seat_manager_authority_instruction;
use phoenix_seat_manager::seat_manager::SeatManager;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::setup::helpers::airdrop;
use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;

#[tokio::test]
async fn test_confirm_renounce_seat_manager_authority_success() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let initiate_renounce_psm_authority_ix =
        create_initiate_renounce_seat_manager_authority_instruction(
            &sdk.client.payer.pubkey(),
            &sdk.active_market_key,
        );

    sdk.client
        .sign_send_instructions(vec![initiate_renounce_psm_authority_ix], vec![])
        .await
        .unwrap();

    let confimr_renounce_psm_authority_ix =
        create_confirm_renounce_seat_manager_authority_instruction(
            &sdk.client.payer.pubkey(),
            &sdk.active_market_key,
        );

    sdk.client
        .sign_send_instructions(vec![confimr_renounce_psm_authority_ix], vec![])
        .await
        .unwrap();

    let (seat_manager_address, _) = get_seat_manager_address(&sdk.active_market_key);
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .unwrap();
    let seat_manager = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data).unwrap();

    assert_eq!(seat_manager.authority, Pubkey::default());

    // Assert that an admin action authorized by the previous authority fails.
    let change_market_status = create_change_market_status_instruction(
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
        MarketStatus::Paused,
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![change_market_status], vec![&sdk.client.payer])
        .await
        .is_err());
}

#[tokio::test]
async fn test_confirm_renounce_seat_manager_authority_fails_unauthorized_signer() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let initiate_renounce_sm_authority_ix =
        create_initiate_renounce_seat_manager_authority_instruction(
            &sdk.client.payer.pubkey(),
            &sdk.active_market_key,
        );

    sdk.client
        .sign_send_instructions(vec![initiate_renounce_sm_authority_ix], vec![])
        .await
        .unwrap();

    let unauthorized_signer = Keypair::new();
    airdrop(&sdk.client, &unauthorized_signer.pubkey(), 1000000000)
        .await
        .unwrap();

    let confirm_renounce_sm_authority_ix =
        create_confirm_renounce_seat_manager_authority_instruction(
            &unauthorized_signer.pubkey(),
            &sdk.active_market_key,
        );

    assert!(sdk
        .client
        .sign_send_instructions(
            vec![confirm_renounce_sm_authority_ix],
            vec![&unauthorized_signer],
        )
        .await
        .is_err());

    let mut confirm_renounce_ix = create_confirm_renounce_seat_manager_authority_instruction(
        &sdk.client.payer.pubkey(),
        &sdk.active_market_key,
    );

    confirm_renounce_ix.accounts[1].is_signer = false;
    let mut new_tx = Transaction::new_with_payer(
        &[confirm_renounce_ix.clone()],
        Some(&unauthorized_signer.pubkey()),
    );

    new_tx.sign(
        &[&unauthorized_signer],
        sdk.client.get_latest_blockhash().await.unwrap(),
    );

    assert!(sdk
        .client
        .client
        .process_transaction(new_tx, &[&unauthorized_signer])
        .await
        .is_err());
}

#[tokio::test]
async fn test_confirm_renounce_seat_manager_authority_fails_if_not_initiated() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority: _,
    } = bootstrap_default(5).await;

    let confirm_renounce_sm_authority_ix =
        create_confirm_renounce_seat_manager_authority_instruction(
            &sdk.client.payer.pubkey(),
            &sdk.active_market_key,
        );

    assert!(sdk
        .client
        .sign_send_instructions(vec![confirm_renounce_sm_authority_ix], vec![])
        .await
        .is_err());
}
