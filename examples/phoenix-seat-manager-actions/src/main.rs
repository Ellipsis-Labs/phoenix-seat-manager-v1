use std::str::FromStr;

use phoenix::state::Side;
use phoenix_sdk::sdk_client::SDKClient;
use phoenix_seat_manager::{
    get_seat_manager_address,
    instruction_builders::{
        create_claim_market_authority_instruction, create_claim_seat_instruction,
    },
    seat_manager::SeatManager,
};
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
};
use spl_token::state::Mint;

#[tokio::main]
async fn main() {
    // Connect to the solana network and get the market address
    let network_url = "https://api.devnet.solana.com";
    let market = Pubkey::from_str("CS2H8nbAVVEUHWPF5extCSymqheQdkd4d7thik6eet9N").unwrap();

    let trader = Keypair::new();
    // Alternatively, read from keypair file for the payer
    // let path = "~/.config/solana/id.json";
    // let auth_payer = read_keypair_file(&*shellexpand::tilde(path)).unwrap();

    let sdk = SDKClient::new(&market, &trader, network_url).await;

    // Only relevant for devnet
    sdk.client
        .request_airdrop(&trader.pubkey(), 1_500_000_000)
        .await
        .unwrap();

    // Setup token accounts if no token accounts exist
    println!("Trader pubkey: {}", trader.pubkey());

    let mut instructions = create_airdrop_spl_ixs(&sdk, &trader.pubkey())
        .await
        .unwrap();

    // Claim seat as a maker on Phoenix and place a limit order
    let claim_seat_ix = create_claim_seat_instruction(&trader.pubkey(), &sdk.active_market_key);
    let place_order_ix = sdk.get_limit_order_ix(sdk.float_price_to_ticks(500.0), Side::Bid, 1_000);
    instructions.extend_from_slice(&[claim_seat_ix, place_order_ix]);

    let sig = sdk
        .client
        .sign_send_instructions(instructions, vec![])
        .await
        .unwrap();

    println!("Tx Signature: {}", sig);
}

// Other examples
async fn claim_market_authority(sdk: &SDKClient, payer: &Keypair) {
    let claim_market_authority_ix =
        create_claim_market_authority_instruction(&sdk.active_market_key, &payer.pubkey());

    let sig = sdk
        .client
        .sign_send_instructions(vec![claim_market_authority_ix], vec![])
        .await
        .unwrap();

    println!("Success! Claimed market authority tx sig: {}", sig);
}

async fn get_seat_manager(sdk: &SDKClient) {
    let seat_manager_address = get_seat_manager_address(&sdk.active_market_key).0;
    let seat_manager_data = sdk
        .client
        .get_account_data(&seat_manager_address)
        .await
        .expect("Failed to get seat manager");

    let seat_manager_struct = bytemuck::try_from_bytes::<SeatManager>(&seat_manager_data)
        .expect("Failed to deserialize seat manager");
    println!("Seat manager: {:?}", seat_manager_struct);
}

// Only needed for devnet testing
pub async fn create_airdrop_spl_ixs(
    sdk_client: &SDKClient,
    recipient_pubkey: &Pubkey,
) -> Option<Vec<Instruction>> {
    // Get base and quote mints from market metadata
    let market_metadata = sdk_client.get_active_market_metadata();
    let base_mint = market_metadata.base_mint;
    let quote_mint = market_metadata.quote_mint;

    let base_mint_account = Mint::unpack(
        &sdk_client
            .client
            .get_account_data(&base_mint)
            .await
            .unwrap(),
    )
    .unwrap();

    let quote_mint_account = Mint::unpack(
        &sdk_client
            .client
            .get_account_data(&quote_mint)
            .await
            .unwrap(),
    )
    .unwrap();

    let quote_mint_authority = quote_mint_account.mint_authority.unwrap();
    let base_mint_authority = base_mint_account.mint_authority.unwrap();

    if sdk_client
        .client
        .get_account(&quote_mint_authority)
        .await
        .unwrap()
        .owner
        != devnet_token_faucet::id()
    {
        return None;
    }

    if sdk_client
        .client
        .get_account(&base_mint_authority)
        .await
        .unwrap()
        .owner
        != devnet_token_faucet::id()
    {
        return None;
    }

    // Get or create the ATA for the recipient. If doesn't exist, create token account
    let mut instructions = vec![];

    let recipient_ata_base =
        spl_associated_token_account::get_associated_token_address(recipient_pubkey, &base_mint);

    if sdk_client
        .client
        .get_account(&recipient_ata_base)
        .await
        .is_err()
    {
        println!("Error retrieving ATA. Creating ATA");
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account(
                &sdk_client.client.payer.pubkey(),
                recipient_pubkey,
                &base_mint,
                &spl_token::id(),
            ),
        )
    };

    let recipient_ata_quote =
        spl_associated_token_account::get_associated_token_address(recipient_pubkey, &quote_mint);

    if sdk_client
        .client
        .get_account(&recipient_ata_quote)
        .await
        .is_err()
    {
        println!("Error retrieving ATA. Creating ATA");
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account(
                &sdk_client.client.payer.pubkey(),
                recipient_pubkey,
                &quote_mint,
                &spl_token::id(),
            ),
        )
    };

    instructions.push(devnet_token_faucet::airdrop_spl_with_mint_pdas_ix(
        &devnet_token_faucet::id(),
        &base_mint,
        &base_mint_authority,
        recipient_pubkey,
        (5000.0 * 1e9) as u64,
    ));

    instructions.push(devnet_token_faucet::airdrop_spl_with_mint_pdas_ix(
        &devnet_token_faucet::id(),
        &quote_mint,
        &quote_mint_authority,
        recipient_pubkey,
        (500000.0 * 1e6) as u64,
    ));

    Some(instructions)
}
