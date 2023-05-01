use borsh::BorshSerialize;
use phoenix::{
    phoenix_log_authority,
    program::{get_seat_address, get_vault_address, status::MarketStatus},
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{
    get_seat_deposit_collector_address, get_seat_manager_address,
    instruction::SeatManagerInstruction,
};

pub struct EvictTraderAccountBackup {
    pub trader_pubkey: Pubkey,
    pub base_token_account_backup: Option<Pubkey>,
    pub quote_token_account_backup: Option<Pubkey>,
}

pub fn create_evict_seat_instruction(
    market: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    signer: &Pubkey,
    traders: Vec<EvictTraderAccountBackup>,
) -> Instruction {
    let (base_vault, _) = get_vault_address(market, base_mint);
    let (quote_vault, _) = get_vault_address(market, quote_mint);
    let (seat_manager, _) = get_seat_manager_address(market);
    let (seat_deposit_collector, _) = get_seat_deposit_collector_address(market);

    let mut accounts = vec![
        AccountMeta::new_readonly(phoenix::id(), false),
        AccountMeta::new_readonly(phoenix_log_authority::id(), false),
        AccountMeta::new(*market, false),
        AccountMeta::new_readonly(seat_manager, false),
        AccountMeta::new(seat_deposit_collector, false),
        AccountMeta::new_readonly(*base_mint, false),
        AccountMeta::new_readonly(*quote_mint, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*signer, true),
    ];

    for trader_accounts in traders.iter() {
        let base_account = get_associated_token_address(&trader_accounts.trader_pubkey, base_mint);
        let quote_account =
            get_associated_token_address(&trader_accounts.trader_pubkey, quote_mint);
        let (seat, _) = get_seat_address(market, &trader_accounts.trader_pubkey);
        accounts.push(AccountMeta::new(trader_accounts.trader_pubkey, false));
        accounts.push(AccountMeta::new(seat, false));
        accounts.push(AccountMeta::new(base_account, false));
        accounts.push(AccountMeta::new(quote_account, false));

        for backup_account in [
            trader_accounts.base_token_account_backup,
            trader_accounts.quote_token_account_backup,
        ]
        .iter()
        {
            if backup_account.is_some() {
                accounts.push(AccountMeta::new(backup_account.unwrap(), false));
            } else {
                accounts.push(AccountMeta::new_readonly(Pubkey::default(), false));
            }
        }
    }

    Instruction {
        program_id: crate::id(),
        accounts,
        data: SeatManagerInstruction::EvictSeat.to_vec(),
    }
}

pub fn create_claim_market_authority_instruction(market: &Pubkey, payer: &Pubkey) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    let (seat_deposit_collector, _) = get_seat_deposit_collector_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(phoenix::id(), false),
            AccountMeta::new_readonly(phoenix_log_authority::id(), false),
            AccountMeta::new(*market, false),
            AccountMeta::new(seat_manager, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new(seat_deposit_collector, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: SeatManagerInstruction::ClaimMarketAuthority.to_vec(),
    }
}

pub fn create_name_seat_manager_successor_instruction(
    authority: &Pubkey,
    market: &Pubkey,
    successor: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(seat_manager, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(*successor, false),
        ],
        data: SeatManagerInstruction::NameSuccessor.to_vec(),
    }
}

pub fn create_claim_seat_manager_authority_instruction(
    market: &Pubkey,
    successor: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(seat_manager, false),
            AccountMeta::new_readonly(*successor, true),
        ],
        data: SeatManagerInstruction::ClaimSeatManagerAuthority.to_vec(),
    }
}

pub fn create_claim_seat_instruction(trader: &Pubkey, market: &Pubkey) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    let (seat_deposit_collector, _) = get_seat_deposit_collector_address(market);
    let (seat, _) = get_seat_address(market, trader);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(phoenix::id(), false),
            AccountMeta::new_readonly(phoenix_log_authority::id(), false),
            AccountMeta::new(*market, false),
            AccountMeta::new(seat_manager, false),
            AccountMeta::new(seat_deposit_collector, false),
            AccountMeta::new_readonly(*trader, false),
            AccountMeta::new(*trader, true),
            AccountMeta::new(seat, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: SeatManagerInstruction::ClaimSeat.to_vec(),
    }
}

pub fn create_claim_seat_authorized_instruction(
    trader: &Pubkey,
    market: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    let (seat_deposit_collector, _) = get_seat_deposit_collector_address(market);
    let (seat, _) = get_seat_address(market, trader);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(phoenix::id(), false),
            AccountMeta::new_readonly(phoenix_log_authority::id(), false),
            AccountMeta::new(*market, false),
            AccountMeta::new(seat_manager, false),
            AccountMeta::new(seat_deposit_collector, false),
            AccountMeta::new_readonly(*trader, false),
            AccountMeta::new(*authority, true),
            AccountMeta::new(seat, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: SeatManagerInstruction::ClaimSeatAuthorized.to_vec(),
    }
}

pub fn create_add_dmm_instruction(
    market: &Pubkey,
    authority: &Pubkey,
    trader: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(*market, false),
            AccountMeta::new(seat_manager, false),
            AccountMeta::new_readonly(*trader, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data: SeatManagerInstruction::AddDesignatedMarketMaker.to_vec(),
    }
}

pub fn create_remove_dmm_instruction(
    market: &Pubkey,
    authority: &Pubkey,
    trader: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(*market, false),
            AccountMeta::new(seat_manager, false),
            AccountMeta::new_readonly(*trader, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data: SeatManagerInstruction::RemoveDesignatedMarketMaker.to_vec(),
    }
}

pub fn create_change_market_status_instruction(
    market: &Pubkey,
    authority: &Pubkey,
    status: MarketStatus,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(phoenix::id(), false),
            AccountMeta::new_readonly(phoenix_log_authority::id(), false),
            AccountMeta::new(*market, false),
            AccountMeta::new(seat_manager, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data: [
            SeatManagerInstruction::ChangeMarketStatus.to_vec(),
            status.try_to_vec().unwrap(),
        ]
        .concat(),
    }
}

pub fn create_name_market_authority_successor_instruction(
    market: &Pubkey,
    authority: &Pubkey,
    successor: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(phoenix::id(), false),
            AccountMeta::new_readonly(phoenix_log_authority::id(), false),
            AccountMeta::new(*market, false),
            AccountMeta::new_readonly(seat_manager, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data: [
            SeatManagerInstruction::NameMarketAuthoritySuccessor.to_vec(),
            successor.to_bytes().to_vec(),
        ]
        .concat(),
    }
}

pub fn create_change_market_fee_recipient_instruction(
    market: &Pubkey,
    authority: &Pubkey,
    new_recipient: &Pubkey,
    quote_mint: &Pubkey,
    current_fee_recipient: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    let quote_account = get_associated_token_address(current_fee_recipient, quote_mint);
    let (quote_vault, _) = get_vault_address(market, quote_mint);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(phoenix::id(), false),
            AccountMeta::new_readonly(phoenix_log_authority::id(), false),
            AccountMeta::new(*market, false),
            AccountMeta::new_readonly(seat_manager, false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(quote_account, false),
            AccountMeta::new(quote_vault, false),
            AccountMeta::new_readonly(*new_recipient, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: SeatManagerInstruction::ChangeMarketFeeRecipient.to_vec(),
    }
}

pub fn create_initiate_renounce_seat_manager_authority_instruction(
    authority: &Pubkey,
    market: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(seat_manager, false),
            AccountMeta::new_readonly(*authority, true),
            // In order to initiate the renunciation of the seat mangaer authority, call name successor and name the system program as the successor.
            AccountMeta::new_readonly(Pubkey::default(), false),
        ],
        data: SeatManagerInstruction::NameSuccessor.to_vec(),
    }
}

pub fn create_confirm_renounce_seat_manager_authority_instruction(
    authority: &Pubkey,
    market: &Pubkey,
) -> Instruction {
    let (seat_manager, _) = get_seat_manager_address(market);
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(seat_manager, false),
            // To confirm the renunciation of the seat manager authority,
            // the current authority authorizes changing the authority of the seat manager to the system program,
            // which was named as the successor in the inititate renounce authority instruction.
            AccountMeta::new_readonly(*authority, true),
        ],
        data: SeatManagerInstruction::ConfirmRenounceSeatManagerAuthority.to_vec(),
    }
}
