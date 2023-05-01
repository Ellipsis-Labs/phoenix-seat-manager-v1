use num_enum::TryFromPrimitive;
use shank::ShankInstruction;

#[repr(u8)]
#[derive(TryFromPrimitive, Debug, Copy, Clone, ShankInstruction, PartialEq, Eq)]
#[rustfmt::skip]
pub enum SeatManagerInstruction {
    /// Claim Authority 
    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, writable, name = "seat_manager", desc = "The seat manager account must sign to claim authority")]
    #[account(4, writable, signer, name = "payer", desc = "Payer account")]
    #[account(5, name = "seat_deposit_collector", desc = "Collects deposits for claiming new seats and refunds for evicting seats")]
    #[account(6, name = "system_program", desc = "System program")]
    ClaimMarketAuthority = 0,

    /// Claim Seat 
    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, writable, name = "seat_manager", desc = "The seat manager account is the market authority")]
    #[account(4, writable, name = "seat_deposit_collector", desc = "Collects deposits for claiming new seats and refunds for evicting seats")]
    #[account(5, signer, name = "trader")]
    #[account(6, writable, signer, name = "payer")]
    #[account(7, writable, name = "seat")]
    #[account(8, name = "system_program", desc = "System program")]
    ClaimSeat = 1,

    /// Claim Seat Authorized
    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, writable, name = "seat_manager", desc = "The seat manager account is the market authority")]
    #[account(4, writable, name = "seat_deposit_collector", desc = "Collects deposits for claiming new seats and refunds for evicting seats")]
    #[account(5, name = "trader")]
    #[account(6, signer, writable, name = "seat_manager_authority", desc = "The seat manager authority account must sign to claim seat")]
    #[account(7, writable, name = "seat")]
    #[account(8, name = "system_program", desc = "System program")]
    ClaimSeatAuthorized = 2,

    /// Evict Seat 
    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, writable, name = "seat_manager", desc = "The seat manager account must sign to evict a seat")]
    #[account(4, writable, name = "seat_deposit_collector", desc = "Collects deposits for claiming new seats and refunds for evicting seats")]
    #[account(5, name = "base_mint")]
    #[account(6, name = "quote_mint")]
    #[account(7, writable, name = "base_vault")]
    #[account(8, writable, name = "quote_vault")]
    #[account(9, name = "associated_token_account_program", desc = "Associated token account program")]
    #[account(10, name = "token_program", desc = "Token program")]
    #[account(11, name = "system program", desc = "System program to handle refund transfers")]
    #[account(12, signer, name = "signer")]
    // There can be multiple traders, so the following pattern can be repeated indefinitely
    #[account(13, writable, name = "trader")]
    #[account(14, name = "seat", desc = "The trader's PDA seat account, seeds are [b'seat', market_address, trader_address]")]
    #[account(15, writable, name = "base_account", desc = "The trader's associated token account for the base mint")]
    #[account(16, writable, name = "quote_account", desc = "The trader's associated token account for the quote mint")]
    #[account(17, writable, name = "base_account_backup", desc = "Non-ATA token account for the base mint, in case the ATA owner is no longer the trader")]
    #[account(18, writable, name = "quote_account_backup", desc = "Non-ATA token account for the quote mint, in case the ATA owner is no longer the trader")]
    EvictSeat = 3,

    /// Add DMM Seat 
    #[account(0, name = "market", desc = "This account holds the market state")]
    #[account(1, writable, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(2, name = "trader")]
    #[account(3, signer, name = "seat_manager_authority", desc = "The seat manager account must sign to create a DMM")]
    AddDesignatedMarketMaker = 4,

    /// Remove DMM Seat 
    #[account(0, name = "market", desc = "This account holds the market state")]
    #[account(1, writable, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(2, name = "trader")]
    #[account(3, signer, name = "seat_manager_authority", desc = "The seat manager authority account must sign to remove a DMM")]
    RemoveDesignatedMarketMaker = 5,

    /// Name Successor 
    #[account(0, writable, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(1, signer, name = "seat_manager_authority", desc = "The seat manager account must sign name a successor")]
    #[account(2, name = "successor", desc = "The new authority account")]
    NameSuccessor = 6,

    /// Claim Seat Manager Authority 
    #[account(0, writable, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(1, signer, name = "successor", desc = "The successor account must sign to claim authority")]
    ClaimSeatManagerAuthority = 7,

    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(4, signer, name = "seat_manager_authority", desc = "The seat manager account must sign change market status")]
    ChangeMarketStatus = 8,
    
    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(4, signer, name = "seat_manager_authority", desc = "The seat manager authority must sign to name a new market authority successor")]
    NameMarketAuthoritySuccessor = 9,
    
    #[account(0, name = "phoenix_program", desc = "Phoenix program")]
    #[account(1, name = "log_authority", desc = "Phoenix log authority")]
    #[account(2, writable, name = "market", desc = "This account holds the market state")]
    #[account(3, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(4, signer, name = "seat_manager_authority", desc = "The seat manager authority must sign to change the fee recipient")]
    #[account(5, writable, name = "current_fee_recipient_quote_token_account", desc = "The current fee recipient's quote token account")]
    #[account(6, writable, name= "quote_vault", desc = "The quote vault account")]
    #[account(7, name = "new_fee_recipient", desc = "Account to become the new recipient of fees")]
    #[account(8, name = "spl_token", desc = "The SPL token program")]
    ChangeMarketFeeRecipient = 10,

    /// Confirm the renunciation of the seat manager authority by setting the SM authority to the system program
    #[account(0, writable, name = "seat_manager", desc = "This account holds the seat manager state")]
    #[account(1, signer, name = "seat_manager_authority", desc = "The seat manager authority must sign to renounce the seat manager authority")]
    ConfirmRenounceSeatManagerAuthority = 11,
}

impl SeatManagerInstruction {
    pub fn to_vec(&self) -> Vec<u8> {
        vec![*self as u8]
    }
}

#[test]
fn test_instruction_serialization() {
    for i in 0..=10 {
        let instruction = SeatManagerInstruction::try_from(i).unwrap();
        assert_eq!(instruction as u8, i);
    }
}
