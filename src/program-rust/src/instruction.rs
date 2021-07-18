use std::convert::{TryFrom, TryInto};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
    msg
};
use arrayref::{array_ref, array_refs};
use crate::error::LockerError;
use crate::types::DESTINATION_CHAIN_ADDRESS_LEN;

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Initialize {
    pub authority: Pubkey
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct LockandMint {
    pub amount: u64,
    pub destination: [u8; DESTINATION_CHAIN_ADDRESS_LEN]
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Release {
    pub amount: u64
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Mint {
    pub amount: u64
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct BurnAndRelease {
    pub amount: u64,
    pub destination: [u8; DESTINATION_CHAIN_ADDRESS_LEN]
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum LockerInstruction {
    Initialize(Initialize),
    LockAndMint(LockandMint),
    Release(Release),
    Mint(Mint),
    BurnAndRelease(BurnAndRelease),
}

impl LockerInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(LockerError::InvalidInstruction)?;
        match tag {
            0 => {
                if rest.len() >= 32usize {
                    let authority_bytes = match <[u8; 32]>::try_from(rest) {
                        Ok(value) => value,
                        Err(_) => return Err(LockerError::InvalidAuthority.into()),
                    };
                    return Ok(Self::Initialize(Initialize{
                        authority: Pubkey::new_from_array(authority_bytes),
                    }));
                }
                Err(LockerError::InvalidAuthority.into())
            }
            1 => {
                if rest.len() >= 8 + DESTINATION_CHAIN_ADDRESS_LEN {
                    let src = array_ref![rest, 0, 8 + DESTINATION_CHAIN_ADDRESS_LEN];
                    let (
                        amount,
                        destination
                    ) = array_refs![src, 8, DESTINATION_CHAIN_ADDRESS_LEN];
                    return Ok(Self::LockAndMint(LockandMint{
                        amount: u64::from_le_bytes(*amount),
                        destination: *destination,
                    }));
                }
                Err(LockerError::InvalidInstruction.into())
            }
            2 => {
                if rest.len() == 8 {
                    return Ok(Self::Release(Release{
                        amount: Self::unpack_amount(rest)?,
                    }));
                }
                return Err(LockerError::InvalidInstruction.into());
            }
            3 => {
                if rest.len() == 8 {
                    return Ok(Self::Mint(Mint{
                        amount: Self::unpack_amount(rest)?,
                    }));
                }
                Err(LockerError::InvalidInstruction.into())
            }
            4 => {
                if rest.len() >= 8 + DESTINATION_CHAIN_ADDRESS_LEN {
                    let src = array_ref![rest, 0, 8 + DESTINATION_CHAIN_ADDRESS_LEN];
                    let (
                        amount,
                        destination
                    ) = array_refs![src, 8, DESTINATION_CHAIN_ADDRESS_LEN];
                    return Ok(Self::BurnAndRelease(BurnAndRelease{
                        amount: u64::from_le_bytes(*amount),
                        destination: *destination,
                    }));
                }
                Err(LockerError::InvalidInstruction.into())
            }
            _ => Err(ProgramError::InvalidInstructionData.into()),
        }
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        let amount = input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(LockerError::InvalidInstruction)?;
        Ok(amount)
    }
}