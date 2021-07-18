/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */

import {
  Account,
  Connection,
  PublicKey,
  LAMPORTS_PER_SOL,
  SystemProgram,
  TransactionInstruction,
  Transaction,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import {TOKEN_PROGRAM_ID} from "@solana/spl-token";
import fs from 'mz/fs';
import path from 'path';
import * as borsh from 'borsh';

import {
  getPayer,
  getRpcUrl,
  longToByteArray,
  newAccountWithLamports,
  readAccountFromFile,
} from './utils';

/**
 * Connection to the network
 */
let connection: Connection;

/**
 * Account (keypair)
 */
let payerAccount: Account;

/**
 *  Test program id
 */
let programId: PublicKey;

/**
 * Path to program files
 */
const PROGRAM_PATH = path.resolve(__dirname, '../../dist/program');

/**
 * Path to program shared object file which should be deployed on chain.
 * This file is created when running either:
 *   - `npm run build:program-c`
 *   - `npm run build:program-rust`
 */
const PROGRAM_SO_PATH = path.join(PROGRAM_PATH, 'test.so');

/**
 * Path to the keypair of the deployed program.
 * This file is created when running `solana program deploy dist/program/test.so`
 */
const PROGRAM_KEYPAIR_PATH = path.join(PROGRAM_PATH, 'test-keypair.json');

/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
  const rpcUrl = await getRpcUrl();
  connection = new Connection(rpcUrl, 'confirmed');
  const version = await connection.getVersion();
  console.log('Connection to cluster established:', rpcUrl, version);
}

/**
 * Establish an account to pay for everything
 */
export async function establishPayer(): Promise<void> {
  let fees = 0;
  if (!payerAccount) {
    const {feeCalculator} = await connection.getRecentBlockhash();

    // Calculate the cost to fund the greeter account
    fees += await connection.getMinimumBalanceForRentExemption(49);

    // Calculate the cost of sending transactions
    fees += feeCalculator.lamportsPerSignature * 100; // wag

    try {
      // Get payer from cli config
      payerAccount = await getPayer();
    } catch (err) {
      // Fund a new payer via airdrop
      payerAccount = await newAccountWithLamports(connection, fees);
    }
  }

  const lamports = await connection.getBalance(payerAccount.publicKey);
  if (lamports < fees) {
    // This should only happen when using cli config keypair
    const sig = await connection.requestAirdrop(
      payerAccount.publicKey,
      fees - lamports,
    );
    await connection.confirmTransaction(sig);
  }

  console.log(
    'Using account',
    payerAccount.publicKey.toBase58(),
    'containing',
    lamports / LAMPORTS_PER_SOL,
    'SOL to pay for fees',
  );
}

/**
 * Check if the Test BPF program has been deployed
 */
export async function checkProgram(): Promise<void> {
  // Read program id from keypair file
  try {
    const programAccount = await readAccountFromFile(PROGRAM_KEYPAIR_PATH);
    programId = programAccount.publicKey;
  } catch (err) {
    const errMsg = (err as Error).message;
    throw new Error(
      `Failed to read program keypair at '${PROGRAM_KEYPAIR_PATH}' due to error: ${errMsg}. Program may need to be deployed with \`solana program deploy dist/program/test.so\``,
    );
  }

  // Check if the program has been deployed
  const programInfo = await connection.getAccountInfo(programId);
  if (programInfo === null) {
    if (fs.existsSync(PROGRAM_SO_PATH)) {
      throw new Error(
        'Program needs to be deployed with `solana program deploy dist/program/test.so`',
      );
    } else {
      throw new Error('Program needs to be built and deployed');
    }
  } else if (!programInfo.executable) {
    throw new Error(`Program is not executable`);
  }
  console.log(`Using program ${programId.toBase58()}`);
}

/**
 * Update value stored in account
 */
export async function initializeProgram(): Promise<void> {
  const byteArray = [0];
  const instrunctionBuffer = Buffer.from(byteArray);
  const authorityBuffer = Buffer.from(payerAccount.publicKey.toBytes());
  const list = [instrunctionBuffer, authorityBuffer];
  const buffer = Buffer.concat(list);

  const accountPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Init")], programId);
  const mintlogPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Mint")], programId);
  const burnlogPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Burn")], programId);
  const systemPubKey = SystemProgram.programId;
  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: payerAccount.publicKey, isSigner: true, isWritable: false},
      {pubkey: accountPubKey[0], isSigner: false, isWritable: true},
      {pubkey: mintlogPubKey[0], isSigner: false, isWritable: true},
      {pubkey: burnlogPubKey[0], isSigner: false, isWritable: true},
      {pubkey: programId, isSigner: false, isWritable: false},
      {pubkey: systemPubKey, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false}
    ],
    programId,
    data: buffer,
  });
  console.log('Sending transaction for update')
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
  );
}

export async function mintToken(): Promise<void> {
  const bs58 = require('bs58');

  const byteArray = [3];
  const amount = longToByteArray(10000);
  const list = [Buffer.from(byteArray), Buffer.from(amount)];
  const buffer = Buffer.concat(list);

  const accountPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Init")], programId);
  const tokenAccountPubKey = new PublicKey('2Fjsu78vgi9FsEMjrTTpfHNRvK1Z9hx4Q6D1dQdog5mS');
  const tokenMintPubKey = new PublicKey('CpM3TgsV6WeJLaaLSpk4AdsZD9AUMdvFZiFUNF5hrzp5');
  console.log('token-mint', tokenMintPubKey.toString());
  console.log('token-account', tokenAccountPubKey);
  console.log('state-account', accountPubKey.toString());

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: payerAccount.publicKey, isSigner: true, isWritable: false},
      {pubkey: accountPubKey[0], isSigner: false, isWritable: true},
      {pubkey: tokenAccountPubKey, isSigner: false, isWritable: true},
      {pubkey: tokenMintPubKey, isSigner: false, isWritable: true},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false}
    ],
    programId,
    data: buffer,
  });
  console.log('Sending transaction for update')
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
  );
}

export async function burnToken(): Promise<void> {
  const byteArray = [4];
  const amount = longToByteArray(10000);
  const ethAddress = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
  const list = [Buffer.from(byteArray), Buffer.from(amount), Buffer.from(ethAddress)];
  const buffer = Buffer.concat(list);

  const ownerAccount = await readAccountFromFile(path.join(path.resolve(__dirname, '../../dist/token-account'), 'token-account-keypair.json'));
  const accountPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Init")], programId);
  const burnlogPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Burn")], programId);
  const tokenAccountPubKey = new PublicKey('2Fjsu78vgi9FsEMjrTTpfHNRvK1Z9hx4Q6D1dQdog5mS');
  const tokenMintPubKey = new PublicKey('CpM3TgsV6WeJLaaLSpk4AdsZD9AUMdvFZiFUNF5hrzp5');
  console.log('token-mint', tokenMintPubKey.toString());
  console.log('token-account', tokenAccountPubKey);
  console.log('state-account', accountPubKey.toString());

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: ownerAccount.publicKey, isSigner: true, isWritable: false},
      {pubkey: accountPubKey[0], isSigner: false, isWritable: true},
      {pubkey: burnlogPubKey[0], isSigner: false, isWritable: true},
      {pubkey: tokenAccountPubKey, isSigner: false, isWritable: true},
      {pubkey: tokenMintPubKey, isSigner: false, isWritable: true},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false}
    ],
    programId,
    data: buffer,
  });
  console.log('Sending transaction for update')
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [ownerAccount],
  );
}

export async function lockToken(): Promise<void> {
  const byteArray = [1];
  const amount = longToByteArray(1000000000);
  const ethAddress = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
  const list = [Buffer.from(byteArray), Buffer.from(amount), Buffer.from(ethAddress)];
  const buffer = Buffer.concat(list);

  const ownerAccount = await readAccountFromFile(path.join(path.resolve(__dirname, '../../dist/token-account'), 'token-account-keypair.json'));
  const accountPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Init")], programId);
  const mintlogPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Mint")], programId);
  const systemPubKey = SystemProgram.programId;

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: ownerAccount.publicKey, isSigner: true, isWritable: false},
      {pubkey: accountPubKey[0], isSigner: false, isWritable: true},
      {pubkey: mintlogPubKey[0], isSigner: false, isWritable: true},
      {pubkey: systemPubKey, isSigner: false, isWritable: false},
    ],
    programId,
    data: buffer,
  });
  console.log('Sending transaction for update')
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [ownerAccount],
  );
}

export async function releaseToken(): Promise<void> {
  const byteArray = [2];
  const amount = longToByteArray(1000000000);
  const list = [Buffer.from(byteArray), Buffer.from(amount)];
  const buffer = Buffer.concat(list);

  const destinationAccount = await readAccountFromFile(path.join(path.resolve(__dirname, '../../dist/token-account'), 'token-account-keypair.json'));
  const accountPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Init")], programId);
  const mintlogPubKey = await PublicKey.findProgramAddress([Buffer.from("Locker"), Buffer.from("Mint")], programId);
  const systemPubKey = SystemProgram.programId;

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: payerAccount.publicKey, isSigner: true, isWritable: false},
      {pubkey: accountPubKey[0], isSigner: false, isWritable: true},
      {pubkey: destinationAccount.publicKey, isSigner: false, isWritable: true},
      {pubkey: systemPubKey, isSigner: false, isWritable: false},
    ],
    programId,
    data: buffer,
  });
  console.log('Sending transaction for update')
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
  );
}
