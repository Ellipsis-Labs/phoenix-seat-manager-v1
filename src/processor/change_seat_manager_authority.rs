use phoenix::program::checkers::Signer;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::loaders::SeatManagerAccount;

pub fn process_name_successor(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let seat_manager = SeatManagerAccount::new(&accounts[0])?;
    let _authority = Signer::new_with_key(&accounts[1], &seat_manager.load()?.authority)?;
    let successor_ai = &accounts[2];

    seat_manager.load_mut()?.successor = *successor_ai.key;
    Ok(())
}

pub fn process_claim_seat_manager_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let seat_manager = SeatManagerAccount::new(&accounts[0])?;
    let successor = Signer::new_with_key(&accounts[1], &seat_manager.load()?.successor)?;

    seat_manager.load_mut()?.authority = *successor.key;
    Ok(())
}
