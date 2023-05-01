use std::mem::size_of;

use phoenix::{
    program::{
        checkers::Signer, create_change_fee_recipient_instruction, create_collect_fees_instruction,
        load_with_dispatch, MarketHeader,
    },
    quantities::WrapperU64,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::{
    get_accounts_for_instruction,
    loaders::{MarketAccount, SeatManagerAccount},
};

pub fn process_change_market_fee_recipient(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    // Get Accounts
    let market_ai = MarketAccount::new(&accounts[2])?;
    let seat_manager = SeatManagerAccount::new_with_market(&accounts[3], market_ai.key)?;
    // Checks that the seat_manager_authority is the valid authority for the seat_manager and a signer
    let _seat_manager_authority =
        Signer::new_with_key(&accounts[4], &seat_manager.load()?.authority)?;
    let current_fee_recipient_quote_token_ai = &accounts[5];
    let new_fee_recipient = &accounts[7];

    let (unclaimed_fees, quote_mint) = {
        // Check if there are unclaimed fees in the market account. If so, generate change fee with unclaimed ix
        let market_data = market_ai.try_borrow_data()?;
        let (header_bytes, market_bytes) = market_data.split_at(size_of::<MarketHeader>());
        let market_header =
            bytemuck::try_from_bytes::<MarketHeader>(header_bytes).map_err(|_| {
                msg!("Invalid market header data");
                ProgramError::InvalidAccountData
            })?;
        let quote_mint = market_header.quote_params.mint_key;
        let market = load_with_dispatch(&market_header.market_size_params, market_bytes)?.inner;
        (market.get_uncollected_fee_amount(), quote_mint)
    };

    if unclaimed_fees.as_u64() > 0 {
        let collect_fee_ix = create_collect_fees_instruction(
            market_ai.key,
            seat_manager.key,
            current_fee_recipient_quote_token_ai.key,
            &quote_mint,
        );

        invoke_signed(
            &collect_fee_ix,
            get_accounts_for_instruction(&collect_fee_ix, accounts)?.as_slice(),
            &[seat_manager
                .seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }

    let ix = create_change_fee_recipient_instruction(
        seat_manager.key,
        market_ai.key,
        new_fee_recipient.key,
    );

    invoke_signed(
        &ix,
        get_accounts_for_instruction(&ix, accounts)?.as_slice(),
        &[seat_manager
            .seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )
}
