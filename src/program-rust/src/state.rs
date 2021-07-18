use solana_program::{
    program_pack::{IsInitialized, Pack, Sealed},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_math::uint::U256;
use crate::types::DESTINATION_CHAIN_ADDRESS_LEN;

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

pub const STATESIZE: usize = 49usize;
pub const LOGSIZE: usize = 32 + DESTINATION_CHAIN_ADDRESS_LEN;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Locker {
    pub is_initialized: bool,
    pub authority: Pubkey,
    pub total_locked: u64,
    pub total_minted: u64,
}

impl Sealed for Locker{}

impl IsInitialized for Locker{
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Locker {
    const LEN: usize = STATESIZE;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Locker::LEN];
        let (
            is_initialized,
            authority,
            total_locked,
            total_minted,
        ) = array_refs![src, 1, 32, 8, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Locker{
            is_initialized,
            authority: Pubkey::new_from_array(*authority),
            total_locked: u64::from_le_bytes(*total_locked),
            total_minted: u64::from_le_bytes(*total_minted)
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Locker::LEN];
        let (
            is_initialized_dst,
            authority_dst,
            total_locked_dst,
            total_minted_dst,
        ) = mut_array_refs![dst, 1, 32, 8, 8];

        let Locker {
            is_initialized,
            authority,
            total_locked,
            total_minted,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        authority_dst.copy_from_slice(authority.as_ref());
        *total_locked_dst = total_locked.to_le_bytes();
        *total_minted_dst = total_minted.to_le_bytes();
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BurnAndReleaseLog {
    pub amount: U256,
    pub recipient: [u8; DESTINATION_CHAIN_ADDRESS_LEN],
}

impl Sealed for BurnAndReleaseLog{}

impl Pack for BurnAndReleaseLog {
    const LEN: usize = LOGSIZE;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, BurnAndReleaseLog::LEN];
        let (
            amount,
            recipient
        ) = array_refs![src, 32, DESTINATION_CHAIN_ADDRESS_LEN];
        Ok(BurnAndReleaseLog{
            amount: U256::from_big_endian(&amount[..]),
            recipient: *recipient,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, BurnAndReleaseLog::LEN];
        let (
            amount_dst,
            recipient_dst
        ) = mut_array_refs![dst, 32, DESTINATION_CHAIN_ADDRESS_LEN];

        let BurnAndReleaseLog {
            amount,
            recipient
        } = self;

        amount.to_big_endian(&mut amount_dst[..]);
        recipient_dst.copy_from_slice(&recipient[..]);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LockAndMintLog {
    pub amount: U256,
    pub recipient: [u8; DESTINATION_CHAIN_ADDRESS_LEN],
}

impl Sealed for LockAndMintLog{}

impl Pack for LockAndMintLog {
    const LEN: usize = LOGSIZE;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, LockAndMintLog::LEN];
        let (
            amount,
            recipient
        ) = array_refs![src, 32, DESTINATION_CHAIN_ADDRESS_LEN];
        Ok(LockAndMintLog{
            amount: U256::from_big_endian(&amount[..]),
            recipient: *recipient,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, LockAndMintLog::LEN];
        let (
            amount_dst,
            recipient_dst
        ) = mut_array_refs![dst, 32, DESTINATION_CHAIN_ADDRESS_LEN];

        let LockAndMintLog {
            amount,
            recipient
        } = self;

        amount.to_big_endian(&mut amount_dst[..]);
        recipient_dst.copy_from_slice(&recipient[..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn rand_bytes(n: usize) -> Vec<u8> {
        let mut output = vec![0u8; n];
        rand::thread_rng().fill_bytes(output.as_mut_slice());
        output
    }

    #[test]
    fn test_burn_log_pack() {
        let amount = rand_bytes(32);
        let mut amount_arr = [0u8; 32];
        amount_arr.copy_from_slice(amount.as_slice());
        let recipient = rand_bytes(25);
        let mut recipient_arr = [0u8; DESTINATION_CHAIN_ADDRESS_LEN];
        recipient_arr[0..25].copy_from_slice(recipient.as_slice());
        let burn_log = BurnAndReleaseLog {
            amount: U256::from_big_endian(amount.as_slice()),
            recipient: recipient_arr,
        };
        let mut burn_log_bytes = [0u8; 64];
        let res = BurnAndReleaseLog::pack(burn_log, &mut burn_log_bytes);
        assert!(res.is_ok());
    }
}