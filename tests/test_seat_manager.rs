use ellipsis_client::program_test::*;
use phoenix_seat_manager::instruction_builders::create_claim_seat_instruction;
use phoenix_seat_manager::instruction_builders::create_evict_seat_instruction;
use phoenix_seat_manager::instruction_builders::EvictTraderAccountBackup;
use rand::seq::IteratorRandom;
use rand::thread_rng;
use solana_program::system_instruction;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::{Keypair, Signer};

mod setup;
use crate::setup::helpers::airdrop;
use crate::setup::init::bootstrap_default;
use crate::setup::init::PhoenixTestClient;

const NUM_SEATS: usize = 1153;

const _INVALID_NUM_SEATS: usize = 1025;

#[tokio::test]
async fn test_seat_manager() {
    let PhoenixTestClient {
        ctx,
        sdk,
        mint_authority: _,
    } = bootstrap_default(0).await;
    // Create 30
    let mut market_traders = vec![];
    for i in 0..NUM_SEATS {
        let t = Keypair::new();
        airdrop(&sdk.client, &t.pubkey(), 100_000_000)
            .await
            .unwrap();
        if i % 100 == 0 {
            let traders = sdk.get_traders().await;
            assert!(traders.len() == i);
            let mut rng = thread_rng();
            let sample = traders
                .iter()
                .map(|tr| EvictTraderAccountBackup {
                    trader_pubkey: *tr.0,
                    base_token_account_backup: None,
                    quote_token_account_backup: None,
                })
                .choose_multiple(&mut rng, 5);
            sdk.client
                .sign_send_instructions(
                    vec![
                        system_instruction::transfer(
                            &sdk.client.payer.pubkey(),
                            &t.pubkey(),
                            1781760,
                        ),
                        create_evict_seat_instruction(
                            &sdk.active_market_key,
                            &sdk.base_mint,
                            &sdk.quote_mint,
                            &t.pubkey(),
                            sample,
                        ),
                    ],
                    vec![&sdk.client.payer, &t],
                )
                .await
                .unwrap();

            // Can't evict a seat while the market is not full
            let traders = sdk.get_traders().await;
            assert!(traders.len() == i);
            println!("Created {} traders", i);
        }
        sdk.client
            .sign_send_instructions(
                vec![
                    system_instruction::transfer(&sdk.client.payer.pubkey(), &t.pubkey(), 1781760),
                    spl_associated_token_account::instruction::create_associated_token_account(
                        &sdk.client.payer.pubkey(),
                        &t.pubkey(),
                        &sdk.base_mint,
                        &spl_token::id(),
                    ),
                    spl_associated_token_account::instruction::create_associated_token_account(
                        &sdk.client.payer.pubkey(),
                        &t.pubkey(),
                        &sdk.quote_mint,
                        &spl_token::id(),
                    ),
                    create_claim_seat_instruction(&t.pubkey(), &sdk.active_market_key),
                ],
                vec![&sdk.client.payer, &t],
            )
            .await
            .unwrap();
        market_traders.push(t);
    }

    let t = Keypair::new();
    let traders = sdk.get_traders().await;
    assert!(traders.len() == NUM_SEATS);
    let mut rng = thread_rng();
    let sample = traders
        .iter()
        .map(|tr| EvictTraderAccountBackup {
            trader_pubkey: *tr.0,
            base_token_account_backup: None,
            quote_token_account_backup: None,
        })
        .choose_multiple(&mut rng, 3);

    // Can evict at most 1 seat when the market is full
    let evict_seat_ix = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &t.pubkey(),
        sample,
    );
    sdk.client
        .sign_send_instructions(
            vec![
                system_instruction::transfer(&ctx.payer.pubkey(), &t.pubkey(), 1781760),
                evict_seat_ix,
            ],
            vec![&sdk.client.payer, &t],
        )
        .await
        .unwrap();
    let traders = sdk.get_traders().await;
    assert!(traders.len() == NUM_SEATS - 1);

    // Seat manager authority can evict multiple seats
    let sample = traders
        .iter()
        .map(|tr| EvictTraderAccountBackup {
            trader_pubkey: *tr.0,
            base_token_account_backup: None,
            quote_token_account_backup: None,
        })
        .choose_multiple(&mut rng, 3);
    let evict_seat_ix = create_evict_seat_instruction(
        &sdk.active_market_key,
        &sdk.base_mint,
        &sdk.quote_mint,
        &sdk.client.payer.pubkey(),
        sample,
    );
    println!("{:?}", evict_seat_ix);
    sdk.client
        .sign_send_instructions(
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                evict_seat_ix,
            ],
            vec![&sdk.client.payer],
        )
        .await
        .unwrap();
    let traders = sdk.get_traders().await;
    assert!(traders.len() == NUM_SEATS - 4);
}
