use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    system_instruction,
    system_program,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_math::uint::U256;
use spl_token;
use std::convert::TryInto;

use crate::{error::LockerError, 
    instruction, 
    instruction::LockerInstruction, 
    state, state::Locker, 
    state::BurnAndReleaseLog, 
    state::LockAndMintLog
};
use crate::types::DESTINATION_CHAIN_ADDRESS_LEN;

pub struct Processor;
impl Processor {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = LockerInstruction::unpack(instruction_data)?;

        match instruction {
            LockerInstruction::Initialize(instruction::Initialize{authority}) => {
                msg!("Instruction: InitEscrow");
                Self::process_init_locker(accounts, authority, program_id)
            }
            LockerInstruction::LockAndMint(instruction::LockandMint{amount, destination}) => {
                msg!("Instruction: LockAndMint");
                Self::process_lock_and_mint(accounts, amount, destination, program_id)
            }
            LockerInstruction::Release(instruction::Release{amount}) => {
                msg!("Instruction: Release");
                Self::process_release(accounts, amount, program_id)
            }
            LockerInstruction::Mint(instruction::Mint{amount}) => {
                msg!("Instruction: Mint");
                Self::process_mint(accounts, amount, program_id)
            }
            LockerInstruction::BurnAndRelease(instruction::BurnAndRelease{amount, destination}) => {
                msg!("Instruction: BurnAndRelease");
                Self::process_burn_and_release(accounts, amount, destination, program_id)
            }
        }
    }

    fn process_init_locker(
        accounts: &[AccountInfo],
        authority: Pubkey,
        program_id: &Pubkey,
    ) -> ProgramResult {  
        let account_info_iter = &mut accounts.iter();
        let initializer_info = next_account_info(account_info_iter)?;
        if !initializer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let state_account_info = next_account_info(account_info_iter)?;
        let mintlog_account_info = next_account_info(account_info_iter)?;
        let burnlog_account_info = next_account_info(account_info_iter)?;

        let program_info = next_account_info(account_info_iter)?;
        if !(program_info.key.eq(program_id)) {
            return Err(ProgramError::InvalidAccountData);
        }
        
        let system_program_info = next_account_info(account_info_iter)?;
        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        msg!("Creating state account pubkey");
        let (state_account_pubkey, nonce) = Pubkey::find_program_address(&[b"Locker", b"Init"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;
        let mut required_balance = rent.minimum_balance(state::STATESIZE);

        let create_state_account_ix = system_instruction::create_account(initializer_info.key, &state_account_pubkey, required_balance, state::STATESIZE as u64, program_id);


        msg!("submitting tx to create program derived state account");
        invoke_signed(
            &create_state_account_ix,
            &[
                initializer_info.clone(),
                state_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[&b"Locker"[..], &b"Init"[..], &[nonce]]],
        )?;
        msg!("state account pubkey: {}", state_account_pubkey);

        msg!("Creating lock and mint log account pubkey");
        let (mintlog_account_pubkey, nonce) = Pubkey::find_program_address(&[b"Locker", b"Mint"], program_id);
        if !(mintlog_account_info.key.eq(&mintlog_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        required_balance = rent.minimum_balance(state::LOGSIZE);

        let create_state_account_ix = system_instruction::create_account(initializer_info.key, &mintlog_account_pubkey, required_balance, state::LOGSIZE as u64, program_id);


        msg!("submitting tx to create program derived state account");
        invoke_signed(
            &create_state_account_ix,
            &[
                initializer_info.clone(),
                mintlog_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[&b"Locker"[..], &b"Mint"[..], &[nonce]]],
        )?;
        msg!("mintlog account pubkey: {}", state_account_pubkey);

        msg!("Creating state account pubkey");
        let (burnlog_account_pubkey, nonce) = Pubkey::find_program_address(&[b"Locker", b"Burn"], program_id);
        if !(burnlog_account_info.key.eq(&burnlog_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let create_state_account_ix = system_instruction::create_account(initializer_info.key,
            &burnlog_account_pubkey,
            required_balance,
            state::LOGSIZE as u64,
            program_id
        );

        msg!("submitting tx to create program derived state account");
        invoke_signed(
            &create_state_account_ix,
            &[
                initializer_info.clone(),
                burnlog_account_info.clone(),
                system_program_info.clone(),
                program_info.clone(),
            ],
            &[&[&b"Locker"[..], &b"Burn"[..], &[nonce]]],
        )?;
        msg!("burn log account pubkey: {}", burnlog_account_pubkey);

        Locker::pack(
            Locker{
                is_initialized: true,
                authority: authority,
                total_locked: 0,
                total_minted: 0
            }, 
            &mut state_account_info.data.borrow_mut()
        )?;

        Ok(())
    }

    fn process_lock_and_mint(
        accounts: &[AccountInfo],
        amount: u64,
        destination: [u8; DESTINATION_CHAIN_ADDRESS_LEN],
        program_id: &Pubkey
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_account_info = next_account_info(account_info_iter)?;
        if !signer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let state_account_info = next_account_info(account_info_iter)?;
        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Locker", b"Init"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mintlog_account_info = next_account_info(account_info_iter)?;
        let (mintlog_account_pubkey, _) = Pubkey::find_program_address(&[b"Locker", b"Mint"], program_id);
        if !(mintlog_account_info.key.eq(&mintlog_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let system_program_info = next_account_info(account_info_iter)?;
        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut state_info = Locker::unpack_unchecked(&state_account_info.data.borrow())?; 
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        }
        state_info.total_locked += amount;
        Locker::pack(state_info, &mut state_account_info.data.borrow_mut())?;

        let transfer_lamports_ix = system_instruction::transfer(
            signer_account_info.key, 
            state_account_info.key, 
            amount
        );

        invoke(
            &transfer_lamports_ix, 
            &[
                signer_account_info.clone(),
                state_account_info.clone(),
                system_program_info.clone()
            ]
        )?;

        let mut log_info = LockAndMintLog::unpack_unchecked(&mintlog_account_info.data.borrow())?;
        log_info.amount = Self::underlying_amount_from_spl_amount(18, 9, amount)?;
        log_info.recipient = destination;
        LockAndMintLog::pack(log_info, &mut mintlog_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn process_release(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_account_info = next_account_info(account_info_iter)?;
        if !signer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let state_account_info = next_account_info(account_info_iter)?;
        let (state_account_pubkey, nonce) = Pubkey::find_program_address(&[b"Locker", b"Init"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut state_info = Locker::unpack_unchecked(&state_account_info.data.borrow())?; 
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        } 
        if !(state_info.authority.eq(signer_account_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }
        state_info.total_locked -= amount;
        Locker::pack(state_info, &mut state_account_info.data.borrow_mut())?;

        let destination_info = next_account_info(account_info_iter)?;

        let system_program_info = next_account_info(account_info_iter)?;
        if !(system_program_info.key.eq(&system_program::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let transfer_lamports_ix = system_instruction::transfer(
            state_account_info.key, 
            destination_info.key, 
            amount
        );

        invoke_signed(
            &transfer_lamports_ix, 
            &[
                signer_account_info.clone(),
                state_account_info.clone(),
                destination_info.clone(),
                system_program_info.clone()
            ],
            &[&[&b"Locker"[..], &b"Init"[..], &[nonce]]],
        )?;

        Ok(())
    }

    fn process_mint(
        accounts: &[AccountInfo],
        amount: u64,
        program_id: &Pubkey
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_account_info = next_account_info(account_info_iter)?;
        if !signer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let state_account_info = next_account_info(account_info_iter)?;
        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Locker", b"Init"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut state_info = Locker::unpack_unchecked(&state_account_info.data.borrow())?; 
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        } 
        if !(state_info.authority.eq(signer_account_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }
        state_info.total_minted += amount;
        Locker::pack(state_info, &mut state_account_info.data.borrow_mut())?;

        let recipient_account_info = next_account_info(account_info_iter)?;

        let minter_info = next_account_info(account_info_iter)?;
        if !(minter_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let token_program_info = next_account_info(account_info_iter)?;
        if !(spl_token::id().eq(token_program_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mint_ix = spl_token::instruction::mint_to(
            token_program_info.key, 
            minter_info.key, 
            recipient_account_info.key, 
            signer_account_info.key, 
            &[signer_account_info.key],
            amount
        )?;
        
        invoke(
            &mint_ix,
            &[
                signer_account_info.clone(),
                minter_info.clone(),
                recipient_account_info.clone(),
                token_program_info.clone(),
            ]
        )?;

        Ok(())
    }

    fn process_burn_and_release(
        accounts: &[AccountInfo],
        amount: u64,
        destination: [u8; DESTINATION_CHAIN_ADDRESS_LEN],
        program_id: &Pubkey
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let signer_account_info = next_account_info(account_info_iter)?;
        if !signer_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let state_account_info = next_account_info(account_info_iter)?;
        let (state_account_pubkey, _) = Pubkey::find_program_address(&[b"Locker", b"Init"], program_id);
        if !(state_account_info.key.eq(&state_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut state_info = Locker::unpack_unchecked(&state_account_info.data.borrow())?; 
        if !state_info.is_initialized(){
            return Err(ProgramError::UninitializedAccount);
        }
        state_info.total_minted -= amount;
        Locker::pack(state_info, &mut state_account_info.data.borrow_mut())?;

        let burnlog_account_info = next_account_info(account_info_iter)?;
        let (burnlog_account_pubkey, _) = Pubkey::find_program_address(&[b"Locker", b"Burn"], program_id);
        if !(burnlog_account_info.key.eq(&burnlog_account_pubkey)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let token_account_info = next_account_info(account_info_iter)?;

        let minter_info = next_account_info(account_info_iter)?;
        if !(minter_info.owner.eq(&spl_token::id())) {
            return Err(ProgramError::InvalidAccountData);
        }

        let token_program_info = next_account_info(account_info_iter)?;
        if !(spl_token::id().eq(token_program_info.key)) {
            return Err(ProgramError::InvalidAccountData);
        }

        let burn_tx = spl_token::instruction::burn(
            token_program_info.key, 
            token_account_info.key, 
            minter_info.key, 
            signer_account_info.key, 
            &[signer_account_info.key], 
            amount
        )?;

        invoke(
            &burn_tx, 
            &[
                signer_account_info.clone(),
                minter_info.clone(),
                token_account_info.clone(),
                token_program_info.clone(),
            ]
        )?;

        let mut log_info = BurnAndReleaseLog::unpack_unchecked(&burnlog_account_info.data.borrow())?;
        log_info.amount = Self::underlying_amount_from_spl_amount(18, 9, amount)?;
        log_info.recipient = destination;
        BurnAndReleaseLog::pack(log_info, &mut burnlog_account_info.data.borrow_mut())?;

        Ok(())
    }

    fn spl_amount_from_underlying_amount(
        underlying_decimals: u8,
        spl_decimals: u8,
        underlying_amount: U256,
    ) -> Result<u64, ProgramError> {
        // the SPL amount would be the same in case no truncating is required.
        if underlying_decimals == spl_decimals {
            return Ok(underlying_amount.as_u64());
        }
        if underlying_decimals > spl_decimals {
            let spl_amount =
                underlying_amount / U256::exp10((underlying_decimals - spl_decimals) as usize);
            return spl_amount
                .try_into()
                .map_err(|_| LockerError::UnexpectedDecimalConversion.into());
        }
        Err(LockerError::UnexpectedDecimalConversion.into())
    }

    fn underlying_amount_from_spl_amount(
        underlying_decimals: u8,
        spl_decimals: u8,
        spl_amount: u64,
    ) -> Result<U256, ProgramError> {
        // the underlying amount would be the same in case no expansion is required.
        if underlying_decimals == spl_decimals {
            return Ok(U256::from(spl_amount));
        }
        if underlying_decimals > spl_decimals {
            let underlying_amount =
                U256::from(spl_amount) * U256::exp10((underlying_decimals - spl_decimals) as usize);
            return Ok(underlying_amount);
        }
        Err(LockerError::UnexpectedDecimalConversion.into())
    }
}
