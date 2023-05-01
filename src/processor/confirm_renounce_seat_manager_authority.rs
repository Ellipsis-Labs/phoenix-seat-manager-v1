use crate::loaders::SeatManagerAccount;
use phoenix::program::{assert_with_msg, checkers::Signer};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process_confirm_renounce_seat_manager_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let seat_manager = SeatManagerAccount::new(&accounts[0])?;
    let _authority = Signer::new_with_key(&accounts[1], &seat_manager.load()?.authority)?;

    let immutable_authority = Pubkey::default();
    assert_with_msg(
        seat_manager.load()?.successor == immutable_authority,
        ProgramError::InvalidAccountData,
        "The successor to the seat manager authority must be the system program to renounce the seat manager authority. Initiate the renounce process by setting the succesor to the system program.",
    )?;

    seat_manager.load_mut()?.authority = immutable_authority;

    Ok(())
}
