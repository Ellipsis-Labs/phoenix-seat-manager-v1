use phoenix::program::create_new_order_instruction;
use phoenix::state::OrderPacket;
use phoenix::state::Side;
use phoenix_seat_manager::get_seat_deposit_collector_address;
use phoenix_seat_manager::instruction_builders::create_claim_seat_authorized_instruction;
use phoenix_seat_manager::instruction_builders::create_claim_seat_instruction;
use solana_program::program_pack::Pack;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

mod setup;
use crate::setup::helpers::airdrop;
use crate::setup::init::bootstrap_default;
use crate::setup::init::setup_account;
use crate::setup::init::PhoenixTestClient;

#[tokio::test]
async fn test_claim_seat_without_trader_as_signer_fails_and_with_signer_succeeds() {
    let PhoenixTestClient {
        ctx: _,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader = setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint)
        .await
        .user;
    let claim_seat_ix = create_claim_seat_instruction(&trader.pubkey(), &sdk.active_market_key);

    // Assert that claim seat fails when trader is not a signer
    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_seat_ix.clone()], vec![])
        .await
        .is_err());

    sdk.client
        .sign_send_instructions(vec![claim_seat_ix], vec![&trader])
        .await
        .unwrap();

    // Asert able to create limit order
    let maker_order_packet = OrderPacket::new_limit_order_default(
        Side::Ask,
        sdk.float_price_to_ticks(10.0),
        1_000_000_000,
    );
    let limit_order_ix = create_new_order_instruction(
        &sdk.active_market_key,
        &trader.pubkey(),
        &sdk.base_mint,
        &sdk.quote_mint,
        &maker_order_packet,
    );
    assert!(sdk
        .client
        .sign_send_instructions(vec![limit_order_ix], vec![&trader])
        .await
        .is_ok());
}

#[tokio::test]
async fn test_claim_seat_authorized_happy_path() {
    let PhoenixTestClient {
        mut ctx,
        sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader = setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint)
        .await
        .user;

    let claim_seat_ix = create_claim_seat_authorized_instruction(
        &trader.pubkey(),
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_seat_ix], vec![])
        .await
        .is_ok());

    // Assert able to create limit order
    let maker_order_packet = OrderPacket::new_limit_order_default(
        Side::Ask,
        sdk.float_price_to_ticks(10.0),
        1_000_000_000,
    );
    let limit_order_ix = create_new_order_instruction(
        &sdk.active_market_key,
        &trader.pubkey(),
        &sdk.base_mint,
        &sdk.quote_mint,
        &maker_order_packet,
    );
    assert!(sdk
        .client
        .sign_send_instructions(vec![limit_order_ix], vec![&trader])
        .await
        .is_ok());

    ctx.warp_to_slot(1203942).unwrap();

    let claim_seat_ix = create_claim_seat_authorized_instruction(
        &trader.pubkey(),
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_seat_ix], vec![])
        .await
        .is_ok());
}

#[tokio::test]
async fn test_claim_seat_authorized_without_authority_signer_fails() {
    let PhoenixTestClient {
        ctx: _,
        mut sdk,
        mint_authority,
    } = bootstrap_default(5).await;

    let trader = setup_account(&sdk.client, &mint_authority, sdk.base_mint, sdk.quote_mint)
        .await
        .user;

    let unauthorized_payer = Keypair::new();
    airdrop(&sdk.client, &unauthorized_payer.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let claim_seat_ix = create_claim_seat_authorized_instruction(
        &trader.pubkey(),
        &sdk.active_market_key,
        &unauthorized_payer.pubkey(),
    );

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_seat_ix], vec![&unauthorized_payer])
        .await
        .is_err());

    let mut claim_seat_ix = create_claim_seat_authorized_instruction(
        &trader.pubkey(),
        &sdk.active_market_key,
        &sdk.client.payer.pubkey(),
    );

    claim_seat_ix.accounts[6].is_signer = false;

    sdk.client.add_keypair(&unauthorized_payer);
    sdk.client.set_payer(&unauthorized_payer.pubkey()).unwrap();

    assert!(sdk
        .client
        .sign_send_instructions(vec![claim_seat_ix], vec![])
        .await
        .is_err());
}

#[tokio::test]
async fn test_claim_seat_deposits_two_token_accounts_lamports() {
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

    let seat_deposit_collector_address =
        get_seat_deposit_collector_address(&sdk.active_market_key).0;
    let seat_deposit_collector_ending_lamports = sdk
        .client
        .get_account(&seat_deposit_collector_address)
        .await
        .unwrap()
        .lamports;

    let expected_deposit_size = 2 * sdk.client.rent_exempt(spl_token::state::Account::LEN);

    assert_eq!(
        seat_deposit_collector_ending_lamports,
        expected_deposit_size
    );
}
