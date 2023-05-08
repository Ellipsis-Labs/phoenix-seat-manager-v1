use ellipsis_macros::declare_id;
use instruction::SeatManagerInstruction;
use processor::{
    process_change_market_status, process_claim_market_authority, process_claim_seat,
    process_claim_seat_manager_authority, process_confirm_renounce_seat_manager_authority,
    process_designated_market_maker, process_evict_seat, process_name_successor,
};
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
};

use crate::processor::{
    process_change_market_fee_recipient, process_name_market_authority_successor,
};
pub mod instruction;
pub mod instruction_builders;
pub mod loaders;
pub mod processor;
pub mod seat_manager;
pub mod shank_structs;

#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Phoenix Seat Manager V1",
    project_url: "https://ellipsislabs.xyz/",
    contacts: "email:maintainers@ellipsislabs.xyz",
    policy: "https://github.com/Ellipsis-Labs/phoenix-v1/blob/master/SECURITY.md",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/Ellipsis-Labs/phoenix-seat-manager-v1",
    auditors: "contact@osec.io"
}

const MAX_DMMS: u64 = 128;

declare_id!("PSMxQbAoDWDbvd9ezQJgARyq6R9L5kJAasaLDVcZwf1");

pub fn get_seat_manager_seeds(
    market: &Pubkey,
    seat_manager: &Pubkey,
    program_id: &Pubkey,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let mut seeds = vec![market.to_bytes().to_vec()];
    let (seat_manager_key, bump) = Pubkey::find_program_address(
        seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice(),
        program_id,
    );
    seeds.push(vec![bump]);

    if seat_manager_key == *seat_manager {
        Ok(seeds)
    } else {
        let caller = std::panic::Location::caller();
        msg!(
            "Invalid seat manager key, expected: {} found {}.\n{}",
            seat_manager_key,
            seat_manager,
            caller
        );
        return Err(ProgramError::InvalidInstructionData.into());
    }
}

pub fn get_seat_manager_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&market.to_bytes()], &crate::id())
}

pub fn get_seat_deposit_collector_seeds(
    market: &Pubkey,
    seat_deposit_collector: &Pubkey,
    program_id: &Pubkey,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let mut seeds = vec![market.to_bytes().to_vec(), b"deposit".to_vec()];
    let (seat_deposit_collector_key, bump) = Pubkey::find_program_address(
        seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice(),
        program_id,
    );
    seeds.push(vec![bump]);

    if seat_deposit_collector_key == *seat_deposit_collector {
        Ok(seeds)
    } else {
        let caller = std::panic::Location::caller();
        msg!(
            "Invalid seat deposit collector key, expected: {} found {}.\n{}",
            seat_deposit_collector_key,
            seat_deposit_collector,
            caller
        );
        return Err(ProgramError::InvalidInstructionData.into());
    }
}

pub fn get_seat_deposit_collector_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&market.to_bytes(), b"deposit"], &crate::id())
}

pub fn get_accounts_for_instruction<'a, 'info>(
    instruction: &Instruction,
    accounts: &'a [AccountInfo<'info>],
) -> Result<Vec<AccountInfo<'info>>, ProgramError> {
    let mut accounts_from_instruction = vec![];
    // This is inefficient, but it also makes life a lot easier
    for account_key in instruction
        .accounts
        .iter()
        .map(|ai| ai.pubkey)
        .chain([instruction.program_id])
    {
        if let Some(account) = accounts.iter().find(|&ai| *ai.key == account_key) {
            accounts_from_instruction.push(account.clone());
        } else {
            msg!("Failed to find key {} for instruction", account_key);
            return Err(ProgramError::InvalidArgument);
        }
    }
    Ok(accounts_from_instruction)
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let instruction =
        SeatManagerInstruction::try_from(*tag).or(Err(ProgramError::InvalidInstructionData))?;

    match instruction {
        SeatManagerInstruction::ClaimMarketAuthority => {
            msg!("SeatManagerInstruction::ClaimMarketAuthority");
            process_claim_market_authority(program_id, accounts)
        }
        SeatManagerInstruction::ClaimSeatAuthorized => {
            msg!("SeatManagerInstruction::ClaimSeatAuthorized");
            process_claim_seat(program_id, accounts, true)
        }
        SeatManagerInstruction::ClaimSeat => {
            msg!("SeatManagerInstruction::ClaimSeat");
            process_claim_seat(program_id, accounts, false)
        }
        SeatManagerInstruction::EvictSeat => {
            msg!("SeatManagerInstruction::EvictSeat");
            process_evict_seat(program_id, accounts)
        }
        SeatManagerInstruction::AddDesignatedMarketMaker => {
            msg!("SeatManagerInstruction::AddDesignatedMarketMaker");
            process_designated_market_maker(program_id, accounts, false)
        }
        SeatManagerInstruction::RemoveDesignatedMarketMaker => {
            msg!("SeatManagerInstruction::RemoveDesignatedMarketMaker");
            process_designated_market_maker(program_id, accounts, true)
        }
        SeatManagerInstruction::NameSuccessor => {
            msg!("SeatManagerInstruction::NameSuccessor");
            process_name_successor(program_id, accounts)
        }
        SeatManagerInstruction::ClaimSeatManagerAuthority => {
            msg!("SeatManagerInstruction::ClaimSeatManagerAuthority");
            process_claim_seat_manager_authority(program_id, accounts)
        }
        SeatManagerInstruction::ChangeMarketStatus => {
            msg!("SeatManagerInstruction::ChangeMarketStatus");
            process_change_market_status(program_id, accounts, data)
        }
        SeatManagerInstruction::NameMarketAuthoritySuccessor => {
            msg!("SeatManagerInstruction::NameMarketAuthoritySuccessor");
            process_name_market_authority_successor(program_id, accounts, data)
        }
        SeatManagerInstruction::ChangeMarketFeeRecipient => {
            msg!("SeatManagerInstruction::ChangeMarketFeeRecipient");
            process_change_market_fee_recipient(program_id, accounts)
        }
        SeatManagerInstruction::ConfirmRenounceSeatManagerAuthority => {
            msg!("SeatManagerInstruction::ConfirmRenounceSeatManagerAuthority");
            process_confirm_renounce_seat_manager_authority(program_id, accounts)
        }
    }
}
