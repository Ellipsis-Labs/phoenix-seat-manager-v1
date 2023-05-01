use bytemuck::{Pod, Zeroable};
use solana_program::{msg, program_error::ProgramError, pubkey::Pubkey};

use crate::MAX_DMMS;

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct SeatManager {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub successor: Pubkey,
    pub num_makers: u64,
    pub _header_padding: [u64; 11],
    pub designated_market_makers: [Pubkey; MAX_DMMS as usize],
    pub _dmm_padding: [u128; MAX_DMMS as usize],
}

impl SeatManager {
    pub fn load(bytes: &'_ [u8]) -> Result<&'_ Self, ProgramError> {
        bytemuck::try_from_bytes::<SeatManager>(bytes).map_err(|_| {
            msg!("Failed to load seat manager from data");
            ProgramError::InvalidAccountData
        })
    }

    pub fn load_mut(bytes: &'_ mut [u8]) -> Result<&'_ mut Self, ProgramError> {
        bytemuck::try_from_bytes_mut::<SeatManager>(bytes).map_err(|_| {
            msg!("Failed to load seat manager from data");
            ProgramError::InvalidAccountData
        })
    }

    pub fn capacity(&self) -> usize {
        self.designated_market_makers.len()
    }

    pub fn contains(&self, trader: &Pubkey) -> bool {
        self.designated_market_makers
            .iter()
            .take(self.num_makers as usize)
            .any(|dmm| dmm == trader)
    }

    pub fn is_full(&self) -> bool {
        self.num_makers == self.designated_market_makers.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.num_makers == 0
    }

    pub fn len(&self) -> usize {
        self.num_makers as usize
    }

    pub fn insert(&mut self, trader: &Pubkey) -> Option<usize> {
        if self.designated_market_makers.contains(trader) {
            msg!("Trader is already a designated market maker");
            return None;
        }
        if self.is_full() {
            msg!("Seat manager is full");
            return None;
        }
        let index = self.num_makers as usize;
        self.designated_market_makers[index] = *trader;
        self.num_makers += 1;
        Some(index)
    }

    /// Performs a swap-remove on the designated market makers array.
    pub fn remove(&mut self, dmm: &Pubkey) -> Option<usize> {
        let index = self
            .designated_market_makers
            .iter()
            .take(self.num_makers as usize)
            .position(|maker| maker == dmm)?;
        let last_index = (self.num_makers - 1) as usize;
        self.designated_market_makers[index] = self.designated_market_makers[last_index];
        self.designated_market_makers[last_index] = Pubkey::default();
        self.num_makers -= 1;
        Some(index)
    }

    pub fn clear_all_dmms(&mut self) {
        for dmm in self.designated_market_makers.iter_mut() {
            *dmm = Pubkey::default();
        }

        self.num_makers = 0;
    }
}
