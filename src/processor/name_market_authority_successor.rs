use borsh::BorshDeserialize;
use phoenix::program::{checkers::Signer, create_name_successor_instruction};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed, pubkey::Pubkey,
};

use crate::{
    get_accounts_for_instruction,
    loaders::{MarketAccount, SeatManagerAccount},
};

pub fn process_name_market_authority_successor(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let market_ai = MarketAccount::new(&accounts[2])?;
    let seat_manager = SeatManagerAccount::new_with_market(&accounts[3], market_ai.key)?;
    // Checks that _authority is the valid authority for the seat_manager and a signer
    let _authority = Signer::new_with_key(&accounts[4], &seat_manager.load()?.authority)?;

    let successor_pubkey = Pubkey::try_from_slice(data)?;

    let name_market_authority_successor_instruction =
        create_name_successor_instruction(seat_manager.key, market_ai.key, &successor_pubkey);

    invoke_signed(
        &name_market_authority_successor_instruction,
        get_accounts_for_instruction(&name_market_authority_successor_instruction, accounts)?
            .as_slice(),
        &[seat_manager
            .seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )
}
