use std::mem::size_of;

use phoenix::program::{checkers::Signer, dispatch_market, MarketHeader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::loaders::{MarketAccount, SeatManagerAccount};

pub fn process_designated_market_maker(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    remove: bool,
) -> ProgramResult {
    let market_ai = MarketAccount::new_with_checked_discriminant(&accounts[0])?;
    let seat_manager_ai = SeatManagerAccount::new_with_market(&accounts[1], market_ai.key)?;
    let trader_ai = &accounts[2];
    // Checks that _authority is the valid authority for the seat_manager and a signer
    let _authority = Signer::new_with_key(&accounts[3], &seat_manager_ai.load()?.authority)?;

    let market_bytes = market_ai.data.borrow();
    let (header_bytes, market_bytes) = market_bytes.split_at(size_of::<MarketHeader>());
    let market_header = bytemuck::try_from_bytes::<MarketHeader>(header_bytes).map_err(|_| {
        msg!("Invalid market header data");
        ProgramError::InvalidAccountData
    })?;
    let market =
        dispatch_market::load_with_dispatch(&market_header.market_size_params, market_bytes)?.inner;

    let registered_traders = market.get_registered_traders();
    if registered_traders.contains(trader_ai.key) {
        if !remove {
            seat_manager_ai
                .load_mut()?
                .insert(trader_ai.key)
                .ok_or_else(|| {
                    msg!("Failed to add trader as DMM");
                    ProgramError::InvalidAccountData
                })?;
        } else {
            seat_manager_ai
                .load_mut()?
                .remove(trader_ai.key)
                .ok_or_else(|| {
                    msg!("Failed to remove trader as DMM, since they are not a DMM");
                    ProgramError::InvalidAccountData
                })?;
        }
    } else {
        msg!("Trader must have a seat on the market");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
