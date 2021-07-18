use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum LockerError {
    /// Invalid instruction
    #[error("Invalid Authority")]
    InvalidAuthority,

    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,

    /// Unexpected conversion
    #[error("Unexpected Decimal Conversion")]
    UnexpectedDecimalConversion,
}

impl From<LockerError> for ProgramError {
    fn from(e: LockerError) -> Self {
        ProgramError::Custom(e as u32)
    }
}