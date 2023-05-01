use phoenix::program::{
    assert_with_msg,
    checkers::{Program, Signer, PDA},
    create_claim_authority_instruction, load_with_dispatch,
    system_utils::create_account,
    MarketHeader, MarketSizeParams,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program,
    sysvar::Sysvar,
};
use std::mem::size_of;

use crate::{
    get_accounts_for_instruction, get_seat_deposit_collector_address,
    loaders::{MarketAccount, SeatManagerAccount},
    seat_manager::SeatManager,
    MAX_DMMS,
};

pub fn process_claim_market_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let market_ai = MarketAccount::new(&accounts[2])?;
    let seat_manager = SeatManagerAccount::new_with_market(&accounts[3], market_ai.key)?;
    let payer = Signer::new(&accounts[4])?;
    let seat_deposit_collector = PDA::new(
        &accounts[5],
        &get_seat_deposit_collector_address(market_ai.key).0,
    )?;
    let system_program = Program::new(&accounts[6], &system_program::id())?;
    {
        let market_bytes = market_ai.data.borrow();
        let (header_bytes, market_data) = market_bytes.split_at(size_of::<MarketHeader>());
        let market_header =
            bytemuck::try_from_bytes::<MarketHeader>(header_bytes).map_err(|_| {
                msg!("Invalid market header data");
                ProgramError::InvalidAccountData
            })?;

        let MarketSizeParams {
            bids_size,
            asks_size,
            num_seats,
        } = market_header.market_size_params;

        assert_with_msg(
            num_seats == bids_size + asks_size + 1 + MAX_DMMS,
            ProgramError::InvalidAccountData,
            &format!(
                "Invalid market size params, bids: {} asks: {} seats: {}.
                Market must have exactly {} more seats than bids and asks",
                bids_size,
                asks_size,
                num_seats,
                MAX_DMMS + 1
            ),
        )?;

        assert_with_msg(
            market_header.successor == *seat_manager.key,
            ProgramError::InvalidArgument,
            &format!("Invalid successor key: {}", market_header.successor),
        )?;

        assert_with_msg(
            market_header.authority != *seat_manager.key,
            ProgramError::InvalidArgument,
            &format!(
                "Seat manager is already the market authority for market: {}",
                market_ai.key
            ),
        )?;

        // Assert that the seat deposit collector account has sufficient lamports equal to the rent of 2 token accounts per trader times the number of existing seats on the market.
        // This is required for seat eviction.
        let market = load_with_dispatch(&market_header.market_size_params, &market_data)?.inner;
        let existing_seats = market.get_registered_traders().len();
        let required_deposits = existing_seats as u64
            * Rent::get()?.minimum_balance(spl_token::state::Account::LEN)
            * 2;
        assert_with_msg(
            seat_deposit_collector.lamports() >= required_deposits,
            ProgramError::InsufficientFunds,
            &format!(
                "Seat deposit collector account does not have enough lamports. Required: {} Actual: {}. Please deposit more lamports to the seat deposit collector account.",
                required_deposits,
                seat_deposit_collector.lamports()
            ),
        )?;
    }

    // Check if seat manager account has already been initialized. If so, clear DMMs. If not, create account.
    if seat_manager.data_is_empty() {
        msg!("Creating and initializing seat manager account");
        create_account(
            &payer,
            &seat_manager,
            &system_program,
            program_id,
            &Rent::get()?,
            size_of::<SeatManager>() as u64,
            seat_manager.seeds.clone(),
        )?;
        let mut seat_manager_data = seat_manager.try_borrow_mut_data()?;
        let seat_manager = SeatManager::load_mut(&mut seat_manager_data)?;
        seat_manager.market = *market_ai.key;
        // The payer of this instruction starts out as the seat manager authority
        seat_manager.authority = *payer.key;
        seat_manager.successor = *payer.key;
    } else {
        let mut seat_manager_struct = seat_manager.load_mut()?;
        assert_with_msg(
            seat_manager_struct.market == *market_ai.key,
            ProgramError::InvalidArgument,
            &format!("Invalid market key: {}", seat_manager_struct.market),
        )?;
        assert_with_msg(
            seat_manager_struct.authority == *payer.key,
            ProgramError::InvalidArgument,
            &format!(
                "Invalid authority signer: {}",
                seat_manager_struct.authority
            ),
        )?;
        seat_manager_struct.clear_all_dmms();
    }

    let claim_authority_instruction =
        create_claim_authority_instruction(seat_manager.key, market_ai.key);
    invoke_signed(
        // This call checks to make sure all of the Phoenix dependencies are met
        // Namely:
        // 1. The downstream program has the correct address
        // 2. The market belongs to the Phoenix program
        // 3. All of the other inputs are supplied correctly
        &claim_authority_instruction,
        // The first 4 accounts are the only ones required by the CPI
        get_accounts_for_instruction(&claim_authority_instruction, accounts)?.as_slice(),
        &[seat_manager
            .seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )
}
