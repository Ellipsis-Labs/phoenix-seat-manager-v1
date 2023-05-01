use ellipsis_client::program_test::*;
use ellipsis_client::EllipsisClient;
use phoenix::program::instruction_builders::*;
use phoenix_seat_manager::get_seat_manager_address;
use phoenix_seat_manager::instruction_builders::create_claim_market_authority_instruction;

use phoenix::program::status::MarketStatus;
use phoenix::program::*;
use phoenix_sdk::sdk_client::SDKClient;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

use crate::setup::helpers::clone_keypair;
use crate::setup::helpers::create_mint;

use super::helpers::airdrop;
use super::helpers::create_associated_token_account;
use super::helpers::mint_tokens;
use super::helpers::sol;

const BOOK_SIZE: usize = 512;
pub const NUM_SEATS: usize = 1153;

const _INVALID_NUM_SEATS: usize = 1025;

pub struct PhoenixTestAccount {
    pub user: Keypair,
    pub base_ata: Pubkey,
    pub quote_ata: Pubkey,
}

pub struct PhoenixTestClient {
    pub ctx: ProgramTestContext,
    pub sdk: SDKClient,
    pub mint_authority: Keypair,
}

pub fn phoenix_test() -> ProgramTest {
    let mut ctx = ProgramTest::new("phoenix_seat_manager", phoenix_seat_manager::id(), None);
    ctx.add_program("phoenix", phoenix::id(), None);
    ctx
}

pub async fn setup_account(
    client: &EllipsisClient,
    authority: &Keypair,
    base_mint: Pubkey,
    quote_mint: Pubkey,
) -> PhoenixTestAccount {
    // initialize user and ATAs
    let user = Keypair::new();
    let base_ata =
        create_associated_token_account(client, &user.pubkey(), &base_mint, &spl_token::id())
            .await
            .unwrap();
    let quote_ata =
        create_associated_token_account(client, &user.pubkey(), &quote_mint, &spl_token::id())
            .await
            .unwrap();

    // airdrop SOL to user
    airdrop(client, &user.pubkey(), sol(10.0)).await.unwrap();

    // airdrop base and quote tokens to user
    mint_tokens(
        client,
        authority,
        &base_mint,
        &base_ata,
        1_000_000 * 1e9 as u64,
        None,
    )
    .await
    .unwrap();

    mint_tokens(
        client,
        authority,
        &quote_mint,
        &quote_ata,
        1_000_000 * 1e6 as u64,
        None,
    )
    .await
    .unwrap();

    PhoenixTestAccount {
        user,
        base_ata,
        quote_ata,
    }
}

pub async fn bootstrap_default(fees_bps: u16) -> PhoenixTestClient {
    bootstrap_with_parameters(100_000, 1_000, 1_000, 9, 6, fees_bps, None, true).await
}

async fn bootstrap_with_parameters(
    num_quote_lots_per_quote_unit: u64,
    num_base_lots_per_base_unit: u64,
    tick_size_in_quote_lots_per_base_unit: u64,
    base_decimals: u8,
    quote_decimals: u8,
    fee_bps: u16,
    raw_base_units_per_base_unit: Option<u32>,
    claim_authority_as_seat_manager: bool,
) -> PhoenixTestClient {
    let context = phoenix_test().start_with_context().await;
    let mut ellipsis_client = EllipsisClient::from_banks(&context.banks_client, &context.payer)
        .await
        .unwrap();
    let authority = Keypair::new();
    ellipsis_client.add_keypair(&authority);
    airdrop(&ellipsis_client, &authority.pubkey(), sol(10.0))
        .await
        .unwrap();
    let market = Keypair::new();
    let params = MarketSizeParams {
        bids_size: BOOK_SIZE as u64,
        asks_size: BOOK_SIZE as u64,
        num_seats: NUM_SEATS as u64,
    };

    // create base and quote token mints
    let base_mint = Keypair::new();
    create_mint(
        &ellipsis_client,
        &authority.pubkey(),
        Some(&authority.pubkey()),
        base_decimals,
        Some(clone_keypair(&base_mint)),
    )
    .await
    .unwrap();

    let quote_mint = Keypair::new();
    create_mint(
        &ellipsis_client,
        &authority.pubkey(),
        Some(&authority.pubkey()),
        quote_decimals,
        Some(clone_keypair(&quote_mint)),
    )
    .await
    .unwrap();

    // initialize default maker and taker accounts
    let maker = setup_account(
        &ellipsis_client,
        &authority,
        base_mint.pubkey(),
        quote_mint.pubkey(),
    )
    .await;
    let taker = setup_account(
        &ellipsis_client,
        &authority,
        base_mint.pubkey(),
        quote_mint.pubkey(),
    )
    .await;

    ellipsis_client.add_keypair(&maker.user);
    ellipsis_client.add_keypair(&taker.user);
    let payer = Keypair::from_bytes(&ellipsis_client.payer.to_bytes()).unwrap();

    create_associated_token_account(
        &ellipsis_client,
        &payer.pubkey(),
        &quote_mint.pubkey(),
        &spl_token::id(),
    )
    .await
    .unwrap();

    let mut init_instructions = vec![];

    init_instructions.extend_from_slice(
        &create_initialize_market_instructions_default(
            &market.pubkey(),
            &base_mint.pubkey(),
            &quote_mint.pubkey(),
            &payer.pubkey(),
            params,
            num_quote_lots_per_quote_unit,
            num_base_lots_per_base_unit,
            tick_size_in_quote_lots_per_base_unit,
            fee_bps,
            raw_base_units_per_base_unit,
        )
        .unwrap(),
    );

    let seat_manager_key = get_seat_manager_address(&market.pubkey()).0;

    init_instructions.push(create_name_successor_instruction(
        &payer.pubkey(),
        &market.pubkey(),
        &seat_manager_key,
    ));
    init_instructions.push(
        phoenix::program::instruction_builders::create_change_market_status_instruction(
            &payer.pubkey(),
            &market.pubkey(),
            MarketStatus::Active,
        ),
    );

    ellipsis_client
        .sign_send_instructions_with_payer(init_instructions, vec![&market])
        .await
        .unwrap();

    if claim_authority_as_seat_manager {
        let ix = create_claim_market_authority_instruction(
            &market.pubkey(),
            &ellipsis_client.payer.pubkey(),
        );
        ellipsis_client
            .sign_send_instructions(vec![ix], vec![&ellipsis_client.payer])
            .await
            .unwrap();
    }

    PhoenixTestClient {
        ctx: context,
        sdk: SDKClient::new_from_ellipsis_client(&market.pubkey(), ellipsis_client).await,
        mint_authority: authority,
    }
}

#[allow(dead_code)]
pub async fn bootstrap_default_without_sm_claiming_authority(fees_bps: u16) -> PhoenixTestClient {
    bootstrap_with_parameters(100_000, 1_000, 1_000, 9, 6, fees_bps, None, false).await
}
