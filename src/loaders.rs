use std::{
    cell::{Ref, RefMut},
    ops::Deref,
};

use phoenix::program::{
    assert_with_msg, checkers::TokenAccountInfo, get_discriminant, MarketHeader,
};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;

use crate::{get_seat_manager_seeds, seat_manager::SeatManager};

pub struct MarketAccount<'a, 'info> {
    pub account: &'a AccountInfo<'info>,
}

impl<'a, 'info> MarketAccount<'a, 'info> {
    pub fn new_with_checked_discriminant(
        account: &'a AccountInfo<'info>,
    ) -> Result<Self, ProgramError> {
        assert_with_msg(
            *account.owner == phoenix::id(),
            ProgramError::InvalidAccountData,
            "Market account must be owned by the Phoenix program",
        )?;

        assert_with_msg(
            !account.data_is_empty(),
            ProgramError::InvalidAccountData,
            "Market account must not be empty",
        )?;
        let data = account.try_borrow_data()?;
        assert_with_msg(
            u64::from_le_bytes(data[..8].try_into().map_err(|_| {
                msg!("Failed to deserialize u64");
                ProgramError::InvalidAccountData
            })?) == get_discriminant::<MarketHeader>()?,
            ProgramError::InvalidAccountData,
            "Market account discriminant mismatch",
        )?;
        Ok(Self { account })
    }

    pub fn new(account: &'a AccountInfo<'info>) -> Result<Self, ProgramError> {
        assert_with_msg(
            *account.owner == phoenix::id(),
            ProgramError::InvalidAccountData,
            "Market account must be owned by the Phoenix program",
        )?;

        assert_with_msg(
            !account.data_is_empty(),
            ProgramError::InvalidAccountData,
            "Market account must not be empty",
        )?;
        Ok(Self { account })
    }
}

impl<'a, 'info> Deref for MarketAccount<'a, 'info> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        self.account
    }
}

pub struct SeatManagerAccount<'a, 'info> {
    pub account: &'a AccountInfo<'info>,
    pub seeds: Vec<Vec<u8>>,
}

impl<'a, 'info> SeatManagerAccount<'a, 'info> {
    pub fn new(account: &'a AccountInfo<'info>) -> Result<Self, ProgramError> {
        let data = account.try_borrow_data()?;
        let seat_manager = SeatManager::load(&data)?;
        let market = seat_manager.market;
        // Assert that the seat manager address is correct
        let seeds = get_seat_manager_seeds(&market, account.key, &crate::id())?;
        Ok(Self { account, seeds })
    }

    pub fn new_with_market(
        account: &'a AccountInfo<'info>,
        market: &Pubkey,
    ) -> Result<Self, ProgramError> {
        // Assert that the seat manager address is correct
        let seeds = get_seat_manager_seeds(market, account.key, &crate::id())?;
        if !account.data_is_empty() {
            let data = account.try_borrow_data()?;
            let seat_manager = SeatManager::load(&data)?;
            if seat_manager.market != *market {
                msg!("Seat manager does not belong to market");
                return Err(ProgramError::InvalidAccountData);
            }
        }
        Ok(Self { account, seeds })
    }

    pub fn load(&self) -> Result<Ref<'_, SeatManager>, ProgramError> {
        let data = self.account.try_borrow_data()?;
        Ok(Ref::map(data, |data| {
            return SeatManager::load(data).unwrap();
        }))
    }

    pub fn load_mut(&self) -> Result<RefMut<'_, SeatManager>, ProgramError> {
        let data = self.account.try_borrow_mut_data()?;
        Ok(RefMut::map(data, |data| {
            return SeatManager::load_mut(data).unwrap();
        }))
    }
}

impl<'a, 'info> Deref for SeatManagerAccount<'a, 'info> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        self.account
    }
}

pub struct AssociatedTokenAccount<'a, 'info> {
    pub account: &'a AccountInfo<'info>,
    pub is_initialized: bool,
    pub has_expected_owner: bool,
}

impl<'a, 'info> AssociatedTokenAccount<'a, 'info> {
    pub fn new(
        account: &'a AccountInfo<'info>,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Self, ProgramError> {
        let is_initialized = !account.data_is_empty();
        let has_expected_owner = if is_initialized {
            // The owner key is found at offset 32 of the token account
            &account.try_borrow_data()?[32..64] == owner.as_ref()
        } else {
            // The owner will match if the account is empty as it will be initialized in this transaction
            true
        };
        assert_with_msg(
            get_associated_token_address(owner, mint) == *account.key,
            ProgramError::InvalidArgument,
            "Associated token account address is incorrect",
        )?;
        Ok(Self {
            account,
            is_initialized,
            has_expected_owner,
        })
    }
}

impl<'a, 'info> AsRef<AccountInfo<'info>> for AssociatedTokenAccount<'a, 'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.account
    }
}

impl<'a, 'info> Deref for AssociatedTokenAccount<'a, 'info> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        self.account
    }
}

pub struct BackupTokenAccount<'a, 'info> {
    pub account: &'a AccountInfo<'info>,
    pub is_supplied: bool,
}

impl<'a, 'info> BackupTokenAccount<'a, 'info> {
    pub fn new(
        account: &'a AccountInfo<'info>,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Self, ProgramError> {
        let is_supplied = *account.owner == spl_token::id();
        if is_supplied {
            TokenAccountInfo::new_with_owner(account, mint, owner)?;
        }
        Ok(Self {
            account,
            is_supplied,
        })
    }
}

impl<'a, 'info> AsRef<AccountInfo<'info>> for BackupTokenAccount<'a, 'info> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.account
    }
}

impl<'a, 'info> Deref for BackupTokenAccount<'a, 'info> {
    type Target = AccountInfo<'info>;

    fn deref(&self) -> &Self::Target {
        self.account
    }
}
