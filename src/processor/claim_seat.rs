use phoenix::program::{
    assert_with_msg,
    checkers::{Signer, PDA},
    create_change_seat_status_instruction, create_request_seat_authorized_instruction,
    get_seat_address,
    status::SeatApprovalStatus,
    Seat,
};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    get_accounts_for_instruction, get_seat_deposit_collector_address,
    loaders::{MarketAccount, SeatManagerAccount},
};

pub fn process_claim_seat(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    authorized: bool,
) -> ProgramResult {
    let market_ai = MarketAccount::new(&accounts[2])?;
    let trader_ai = &accounts[5];
    let seat_ai = PDA::new(
        &accounts[7],
        &get_seat_address(market_ai.key, trader_ai.key).0,
    )?;
    if !seat_ai.data_is_empty() {
        // If the seat is already Approved and exists on the market, we can return Ok(())
        let seat_data = &seat_ai.try_borrow_data()?;
        let seat_struct = bytemuck::from_bytes::<Seat>(&seat_data);
        if SeatApprovalStatus::from(seat_struct.approval_status) == SeatApprovalStatus::Approved {
            return Ok(());
        }
    }

    let seat_manager = SeatManagerAccount::new_with_market(&accounts[3], market_ai.key)?;
    let seat_deposit_collector = PDA::new(
        &accounts[4],
        &get_seat_deposit_collector_address(market_ai.key).0,
    )?;
    let payer = Signer::new(&accounts[6])?;

    if !authorized {
        assert_with_msg(
            trader_ai.is_signer,
            ProgramError::MissingRequiredSignature,
            "Trader must sign",
        )?;
    } else {
        assert_with_msg(
            *payer.key == seat_manager.load()?.authority,
            ProgramError::MissingRequiredSignature,
            "If authorized, the payer must be the seat manager's authority",
        )?;
    }

    if seat_ai.data_is_empty() {
        let request_seat_instruction = create_request_seat_authorized_instruction(
            seat_manager.key,
            payer.key,
            market_ai.key,
            trader_ai.key,
        );
        invoke_signed(
            &request_seat_instruction,
            get_accounts_for_instruction(&request_seat_instruction, accounts)?.as_slice(),
            &[seat_manager
                .seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }

    // A deposit equal to the rent of two token accounts is required to mitigate closing of token accounts prior to eviction.
    // If there were no deposit, an attacker can claim a seat, close the token accounts, force the creation of new token accounts by the evicting party, and finally close those token accounts to claim the rent.
    let minimum_rent_for_token_account =
        Rent::get()?.minimum_balance(spl_token::state::Account::LEN);

    let deposit_amount = minimum_rent_for_token_account * 2;

    let deposit_ix =
        system_instruction::transfer(payer.key, seat_deposit_collector.key, deposit_amount);

    invoke(
        &deposit_ix,
        get_accounts_for_instruction(&deposit_ix, accounts)?.as_slice(),
    )?;

    // Note the seat must be in a NotApproved state for this to work
    let change_seat_status_instruction = create_change_seat_status_instruction(
        seat_manager.key,
        market_ai.key,
        trader_ai.key,
        SeatApprovalStatus::Approved,
    );
    invoke_signed(
        &change_seat_status_instruction,
        get_accounts_for_instruction(&change_seat_status_instruction, accounts)?.as_slice(),
        &[seat_manager
            .seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )
}
