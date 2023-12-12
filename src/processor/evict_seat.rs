use std::{mem::size_of, slice::Iter};

use crate::{
    get_accounts_for_instruction, get_seat_deposit_collector_address,
    get_seat_deposit_collector_seeds,
    loaders::{AssociatedTokenAccount, BackupTokenAccount, MarketAccount, SeatManagerAccount},
};
use itertools::{Chunk, Itertools};
use phoenix::{
    program::{
        assert_with_msg,
        checkers::{MintAccountInfo, Program, Signer, PDA},
        create_change_seat_status_instruction, create_evict_seat_instruction, dispatch_market,
        status::SeatApprovalStatus,
        MarketHeader, MarketSizeParams,
    },
    state::TraderState,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent,
    system_instruction, system_program, sysvar::Sysvar,
};
use spl_associated_token_account::instruction::create_associated_token_account;

struct TraderAccountsContext<'a, 'info> {
    trader: &'a AccountInfo<'info>,
    _seat: &'a AccountInfo<'info>,
    base_associated_token_account: AssociatedTokenAccount<'a, 'info>,
    quote_associated_token_account: AssociatedTokenAccount<'a, 'info>,
    backup_base_token_account: BackupTokenAccount<'a, 'info>,
    backup_quote_token_account: BackupTokenAccount<'a, 'info>,
}

impl<'a, 'info> TraderAccountsContext<'a, 'info> {
    pub fn load_from_chunk_iter(
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        mut account_iter: Chunk<'a, Iter<'a, AccountInfo<'info>>>,
    ) -> Result<Self, ProgramError> {
        let trader = account_iter.next().ok_or_else(|| {
            msg!("Missing trader account");
            ProgramError::NotEnoughAccountKeys
        })?;
        let trader_key = *trader.key;
        Ok(Self {
            trader,
            _seat: account_iter.next().ok_or_else(|| {
                msg!("Missing seat account");
                ProgramError::NotEnoughAccountKeys
            })?,
            base_associated_token_account: account_iter
                .next()
                .ok_or_else(|| {
                    msg!("Missing base account");
                    ProgramError::NotEnoughAccountKeys
                })
                .and_then(|ai| AssociatedTokenAccount::new(ai, base_mint, &trader_key))?,
            quote_associated_token_account: account_iter
                .next()
                .ok_or_else(|| {
                    msg!("Missing quote account");
                    ProgramError::NotEnoughAccountKeys
                })
                .and_then(|ai| AssociatedTokenAccount::new(ai, quote_mint, &trader_key))?,
            backup_base_token_account: account_iter
                .next()
                .ok_or_else(|| {
                    msg!("Missing backup base token account");
                    ProgramError::NotEnoughAccountKeys
                })
                .and_then(|ai| BackupTokenAccount::new(ai, base_mint, &trader_key))?,
            backup_quote_token_account: account_iter
                .next()
                .ok_or_else(|| {
                    msg!("Missing backup quote token account");
                    ProgramError::NotEnoughAccountKeys
                })
                .and_then(|ai| BackupTokenAccount::new(ai, quote_mint, &trader_key))?,
        })
    }
}

pub fn process_evict_seat(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let market_ai = MarketAccount::new(&accounts[2])?;
    let seat_manager = SeatManagerAccount::new_with_market(&accounts[3], market_ai.key)?;
    let seat_deposit_collector = PDA::new(
        &accounts[4],
        &get_seat_deposit_collector_address(market_ai.key).0,
    )?;
    let system_program = Program::new(&accounts[11], &system_program::id())?;
    let signer = Signer::new(&accounts[12])?;

    // Assert valid mint accounts and signer
    let base_mint_ai = MintAccountInfo::new(&accounts[5])?;
    let quote_mint_ai = MintAccountInfo::new(&accounts[6])?;

    // Retrieve seat manager seeds and check if signer is authorized
    let is_fully_authorized = *signer.key == seat_manager.load()?.authority;

    // Get market parameters to perform checks
    let (base_mint, quote_mint, market_size_params, has_partial_eviction_privileges) = {
        let market_bytes = market_ai.data.borrow();
        let (header_bytes, market_bytes) = market_bytes.split_at(size_of::<MarketHeader>());
        let market_header =
            bytemuck::try_from_bytes::<MarketHeader>(header_bytes).map_err(|_| {
                msg!("Invalid market header data");
                ProgramError::InvalidAccountData
            })?;
        let (base_mint, quote_mint) = (
            market_header.base_params.mint_key,
            market_header.quote_params.mint_key,
        );
        let market =
            dispatch_market::load_with_dispatch(&market_header.market_size_params, market_bytes)?
                .inner;

        let registered_traders = market.get_registered_traders();

        // When this boolean is true, it gives the signer the privilege to evict any seat with 0 locked base lots and 0 locked quote lots
        let has_partial_eviction_privileges =
            registered_traders.capacity() == registered_traders.len() && !is_fully_authorized;

        assert_with_msg(
            base_mint_ai.info.key == &base_mint,
            ProgramError::InvalidAccountData,
            "Base mint mismatch",
        )?;
        assert_with_msg(
            quote_mint_ai.info.key == &quote_mint,
            ProgramError::InvalidAccountData,
            "Quote mint mismatch",
        )?;
        (
            base_mint,
            quote_mint,
            market_header.market_size_params,
            has_partial_eviction_privileges,
        )
    };

    // Perform eviction for trader(s)
    for trader_accounts in &accounts[13..].iter().chunks(6) {
        let TraderAccountsContext {
            trader: trader_ai,
            _seat,
            base_associated_token_account,
            quote_associated_token_account,
            backup_base_token_account,
            backup_quote_token_account,
        } = TraderAccountsContext::load_from_chunk_iter(&base_mint, &quote_mint, trader_accounts)?;

        // Check if trader is a DMM; if so, continue (cannot evict a DMM)
        if seat_manager.load()?.contains(trader_ai.key) {
            continue;
        }

        // Retrieve trader state if available and ensure no lots are locked before performing eviction-related actions
        if let Some(trader_state) =
            retrieve_trader_state(&market_ai, &market_size_params, trader_ai)?
        {
            // If a trader has 0 balances in the Phoenix program, then anyone can remove it
            let seat_is_empty = trader_state.base_lots_locked == 0
                && trader_state.quote_lots_locked == 0
                && trader_state.base_lots_free == 0
                && trader_state.quote_lots_free == 0;

            let can_evict_trader = if has_partial_eviction_privileges || is_fully_authorized {
                trader_state.base_lots_locked == 0 && trader_state.quote_lots_locked == 0
            } else {
                seat_is_empty
            };

            if can_evict_trader {
                // Change seat status
                change_seat_status_not_approved_cpi(
                    &market_ai,
                    &seat_manager,
                    trader_ai,
                    accounts,
                    seat_manager.seeds.clone(),
                )?;

                // Check ATAs for base and quote; if doesn't exist, create ATAs, using the deposit collected at the time of claim seat.
                // If ATAs are already created and belong to the traders, then refund the deposit collected at the time of claim seat back to the trader
                // In the edge case that the ATA is created but does not belong to the trader, refund the deposit to the signer, the likely creator and rent-payer of the backup token accounts.
                let seat_deposit_collector_seeds = get_seat_deposit_collector_seeds(
                    market_ai.key,
                    seat_deposit_collector.key,
                    program_id,
                )?;
                let mut total_trader_refund = 0;
                let mut total_signer_refund = 0;

                for (associated_token_account, token_mint) in [
                    (&base_associated_token_account, base_mint),
                    (&quote_associated_token_account, quote_mint),
                ] {
                    // Calculate appropriate refund amounts and recipients based on ATA context
                    let (trader_refund, signer_refund) = get_trader_and_signer_refund_amounts(
                        associated_token_account.is_initialized,
                        associated_token_account.has_expected_owner,
                    )?;

                    total_trader_refund += trader_refund;
                    total_signer_refund += signer_refund;

                    create_ata_if_needed(
                        trader_ai,
                        &token_mint,
                        associated_token_account,
                        &seat_deposit_collector,
                        seat_deposit_collector_seeds.clone(),
                        accounts,
                    )?;
                }

                let minimum_rent_for_token_account =
                    Rent::get()?.minimum_balance(spl_token::state::Account::LEN);

                assert_with_msg(
                    total_trader_refund + total_signer_refund <= minimum_rent_for_token_account * 2,
                    ProgramError::InvalidAccountData,
                    "Total refund cannot exceed rent for two token accounts. Check token account inputs."
                )?;

                // Handle refunds if any to trader and signer
                handle_refund(
                    total_trader_refund,
                    trader_ai,
                    &seat_deposit_collector,
                    seat_deposit_collector_seeds.clone(),
                    &system_program,
                )?;
                handle_refund(
                    total_signer_refund,
                    &signer,
                    &seat_deposit_collector,
                    seat_deposit_collector_seeds,
                    &system_program,
                )?;

                // Evict seat for trader
                let evict_seat_cpi_context = EvictSeatCpiContext {
                    base_mint,
                    quote_mint,
                    base_ata_owner_match: base_associated_token_account.has_expected_owner,
                    quote_ata_owner_match: quote_associated_token_account.has_expected_owner,
                    seat_manager_signer_seeds: seat_manager.seeds.clone(),
                };
                evict_seat_cpi(
                    &seat_manager,
                    &market_ai,
                    trader_ai,
                    &backup_base_token_account,
                    &backup_quote_token_account,
                    accounts,
                    &evict_seat_cpi_context,
                )?;

                // If the signer is not fully authorized and if the currently evicted seat is not empty, only one eviction is allowed at a time
                if !is_fully_authorized && !seat_is_empty {
                    msg!("Successfully evicted 1 seat");
                    break;
                }
            }
        }
    }
    Ok(())
}

pub fn retrieve_trader_state(
    market_ai: &AccountInfo,
    market_size_params: &MarketSizeParams,
    trader_ai: &AccountInfo,
) -> Result<Option<TraderState>, ProgramError> {
    let bytes = market_ai.data.borrow();
    let (_, market_bytes) = bytes.split_at(size_of::<MarketHeader>());
    let market = dispatch_market::load_with_dispatch(market_size_params, market_bytes)?.inner;
    let registered_traders = market.get_registered_traders();
    Ok(registered_traders.get(trader_ai.key).copied())
}

pub fn change_seat_status_not_approved_cpi(
    market: &AccountInfo,
    seat_manager: &AccountInfo,
    trader: &AccountInfo,
    accounts: &[AccountInfo],
    seat_manager_seeds: Vec<Vec<u8>>,
) -> ProgramResult {
    let change_seat_status_instruction = create_change_seat_status_instruction(
        seat_manager.key,
        market.key,
        trader.key,
        SeatApprovalStatus::NotApproved,
    );
    let change_seat_accounts =
        match get_accounts_for_instruction(&change_seat_status_instruction, accounts) {
            Ok(change_seat_accounts) => change_seat_accounts,
            Err(_) => return Err(ProgramError::InvalidAccountData),
        };

    invoke_signed(
        &change_seat_status_instruction,
        change_seat_accounts.as_slice(),
        &[seat_manager_seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;

    Ok(())
}

pub fn create_ata_if_needed(
    trader_ai: &AccountInfo,
    mint: &Pubkey,
    ata: &AssociatedTokenAccount,
    payer: &AccountInfo,
    payer_seeds: Vec<Vec<u8>>,
    accounts: &[AccountInfo],
) -> ProgramResult {
    if !ata.is_initialized {
        msg!("Creating ATA for base token");
        create_associated_token_account_with_cpi(
            payer.key,
            trader_ai.key,
            mint,
            payer_seeds,
            accounts,
        )?;
    }
    Ok(())
}

pub struct EvictSeatCpiContext {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_ata_owner_match: bool,
    pub quote_ata_owner_match: bool,
    pub seat_manager_signer_seeds: Vec<Vec<u8>>,
}

pub fn evict_seat_cpi(
    seat_manager_ai: &AccountInfo,
    market_ai: &AccountInfo,
    trader_ai: &AccountInfo,
    backup_base_token_account: &BackupTokenAccount,
    backup_quote_token_account: &BackupTokenAccount,
    accounts: &[AccountInfo],
    evict_seat_cpi_context: &EvictSeatCpiContext,
) -> ProgramResult {
    // Then evict the seat with the appropriate token accounts
    let mut evict_seat_instruction = create_evict_seat_instruction(
        seat_manager_ai.key,
        market_ai.key,
        trader_ai.key,
        &evict_seat_cpi_context.base_mint,
        &evict_seat_cpi_context.quote_mint,
    );

    // If the ATA owner gets reassigned to an address other than the trader, backup token accounts can be passed in to be used in the evict seat instruction.
    // The rationale is to prevent sending of token to a token account that is not owned by the trader.
    if !evict_seat_cpi_context.base_ata_owner_match {
        assert_with_msg(
            backup_base_token_account.is_supplied,
            ProgramError::InvalidArgument,
            "Backup base token account is not supplied",
        )?;
        evict_seat_instruction.accounts[6].pubkey = *backup_base_token_account.key;
        msg!(
            "Using backup base token account: {}",
            backup_base_token_account.key
        );
    }

    if !evict_seat_cpi_context.quote_ata_owner_match {
        assert_with_msg(
            backup_quote_token_account.is_supplied,
            ProgramError::InvalidArgument,
            "Backup base token account is not supplied",
        )?;
        evict_seat_instruction.accounts[7].pubkey = *backup_quote_token_account.key;
        msg!(
            "Using backup quote token account: {}",
            backup_quote_token_account.key
        );
    }

    let evict_seat_accounts = match get_accounts_for_instruction(&evict_seat_instruction, accounts)
    {
        Ok(evict_seat_accounts) => evict_seat_accounts,
        Err(_) => return Err(ProgramError::InvalidAccountData),
    };
    invoke_signed(
        &evict_seat_instruction,
        evict_seat_accounts.as_slice(),
        &[evict_seat_cpi_context
            .seat_manager_signer_seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )
}

pub fn create_associated_token_account_with_cpi(
    signer: &Pubkey,
    trader: &Pubkey,
    mint: &Pubkey,
    signer_seeds: Vec<Vec<u8>>,
    account_infos: &[AccountInfo],
) -> ProgramResult {
    let create_base_ata_instruction =
        create_associated_token_account(signer, trader, mint, &spl_token::ID);

    let account_infos_needed =
        get_accounts_for_instruction(&create_base_ata_instruction, account_infos)?;

    invoke_signed(
        &create_base_ata_instruction,
        account_infos_needed.as_slice(),
        &[signer_seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;
    Ok(())
}

fn get_trader_and_signer_refund_amounts(
    ata_is_initialized: bool,
    ata_has_expected_owner: bool,
) -> Result<(u64, u64), ProgramError> {
    let token_account_rent_fee = Rent::get()?.minimum_balance(spl_token::state::Account::LEN);
    let mut trader_refund = 0;
    let mut signer_refund = 0;

    match (ata_is_initialized, ata_has_expected_owner) {
        // If the ATA is not initialized, the rent fee is spent by the seat deposit collector
        (false, _) => (),
        // Most likely outcome: the ATA is initialized and the owner matches the trader
        (true, true) => trader_refund += token_account_rent_fee,
        // Least likely outcome: the ATA is initialized but the owner does not match the trader
        (true, false) => signer_refund += token_account_rent_fee,
    }
    Ok((trader_refund, signer_refund))
}

fn handle_refund<'a>(
    refund_amount: u64,
    refund_destination: &AccountInfo<'a>,
    seat_deposit_collector: &AccountInfo<'a>,
    seat_deposit_collecto_seeds: Vec<Vec<u8>>,
    system_program: &AccountInfo<'a>,
) -> ProgramResult {
    if refund_amount > 0 {
        msg!(
            "Refunding {} lamports to {}",
            refund_amount,
            refund_destination.key
        );

        let transfer_ix = system_instruction::transfer(
            seat_deposit_collector.key,
            refund_destination.key,
            refund_amount,
        );
        invoke_signed(
            &transfer_ix,
            &[
                seat_deposit_collector.clone(),
                refund_destination.clone(),
                system_program.clone(),
            ],
            &[seat_deposit_collecto_seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }
    Ok(())
}
