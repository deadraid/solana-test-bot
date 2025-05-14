use crate::bench::Bench;
use crate::config::PingThingsArgs;
use crate::core::extract_instructions;
use crate::meteora::constants::{init_pool_indices as idx, METEORA_PROGRAM_ID, WSOL_MINT};
use crate::meteora::types::{MeteoraSwapParams, TradeDirection};

use crate::meteora::constants::INIT_POOL_DISCRIM;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatusMeta;
use std::str::FromStr;
use tracing::debug;

/// Controller that listens to Meteora pool initialization and triggers a buy once WSOL liquidity appears.
pub struct MeteoraController {
    config: PingThingsArgs,
    bench: Bench,
    /// Prevents sending multiple buy transactions for the first token.
    is_buy: bool,
    /// Cache of already-seen mints so we do not react twice.
    seen_mints: std::collections::HashSet<Pubkey>,
}

impl MeteoraController {
    pub fn new(config: PingThingsArgs, bench: Bench) -> Self {
        Self {
            config,
            bench,
            is_buy: false,
            seen_mints: std::collections::HashSet::new(),
        }
    }

    /// Handles every transaction pushed from Yellowstone Geyser.
    pub async fn transaction_handler(
        &mut self,
        _signature: solana_sdk::signature::Signature,
        transaction: VersionedTransaction,
        meta: TransactionStatusMeta,
        _is_vote: bool,
        _slot: u64,
    ) -> anyhow::Result<()> {
        debug!(
            "[LOG_HANDLER] MeteoraController::transaction_handler called for sig: {:?}",
            _signature
        );

        if self.is_buy {
            debug!("[LOG_HANDLER] Already bought, exiting handler.");
            // Already bought, ignore further processing.
            return Ok(());
        }

        let instructions = extract_instructions(meta.clone(), transaction.clone())?;
        debug!(
            "[LOG_HANDLER] Extracted {} instructions.",
            instructions.len()
        );

        let program_id = Pubkey::from_str(METEORA_PROGRAM_ID)?;
        let target_instruction_opt = instructions.iter().find(|inst| {
            if inst.program_id != program_id {
                return false;
            }
            // Log details for instructions from the Meteora program
            debug!("[LOG_HANDLER_FIND] Checking Meteora instruction. Accounts: {}, Data len: {}, Program ID: {}", inst.accounts.len(), inst.data.len(), inst.program_id);
            if inst.data.len() >= 8 {
                debug!("[LOG_HANDLER_FIND] Data prefix (first 8 bytes): {:?}", &inst.data[0..8]);
            } else {
                debug!("[LOG_HANDLER_FIND] Data (less than 8 bytes): {:?}", &inst.data);
            }
            
            let data = &inst.data;
            let matches_v2 = data.starts_with(&INIT_POOL_DISCRIM);
            // Assuming INIT_POOL_DISCRIM_V1 is correctly defined in constants
            let matches_v1 = data.starts_with(&crate::meteora::constants::INIT_POOL_DISCRIM_V1); 
            
            if matches_v1 || matches_v2 {
                debug!("[LOG_HANDLER_FIND] Found a match! V1: {}, V2: {}", matches_v1, matches_v2);
            }

            matches_v1 || matches_v2
        });
        let target_instruction = match target_instruction_opt {
            Some(i) => {
                debug!("[LOG_HANDLER] Found target Meteora instruction.");
                i
            }
            None => {
                debug!("[LOG_HANDLER] Target Meteora instruction not found, exiting handler.");
                return Ok(());
            }
        };

        // Pull token mints from account list.
        let token_a_mint = target_instruction.accounts[idx::TOKEN_A_MINT].pubkey;
        let token_b_mint = target_instruction.accounts[idx::TOKEN_B_MINT].pubkey;

        debug!(
            "[LOG_HANDLER] Checking token pair for WSOL. Token A: {}, Token B: {}",
            token_a_mint, token_b_mint
        );
        // React only when WSOL is one of the pair.
        let (other_token_mint, direction, protocol_fee_acc) =
            if token_a_mint == Pubkey::from_str(WSOL_MINT)? {
                (
                    token_b_mint,
                    TradeDirection::AtoB,
                    target_instruction.accounts[idx::PROTOCOL_TOKEN_A_FEE].pubkey,
                )
            } else if token_b_mint == Pubkey::from_str(WSOL_MINT)? {
                (
                    token_a_mint,
                    TradeDirection::BtoA,
                    target_instruction.accounts[idx::PROTOCOL_TOKEN_B_FEE].pubkey,
                )
            } else {
                debug!("[LOG_HANDLER] Not a WSOL pool, exiting handler.");
                return Ok(()); // Not a WSOL pool.
            };

        debug!(
            "[LOG_HANDLER] Checking if mint {} was seen before...",
            other_token_mint
        );
        // Check first appearance.
        if !self.seen_mints.insert(other_token_mint) {
            debug!(
                "[LOG_HANDLER] Mint {} was already seen, exiting handler.",
                other_token_mint
            );
            return Ok(());
        }

        debug!(
            "Detected first WSOL liquidity for mint {} in pool {}",
            other_token_mint,
            target_instruction.accounts[idx::POOL].pubkey
        );

        // Construct swap params.
        let owner_keypair = Keypair::from_base58_string(&self.config.private_key);
        let owner = owner_keypair.pubkey();
        let user_source = spl_associated_token_account::get_associated_token_address(
            &owner,
            &Pubkey::from_str(WSOL_MINT)?,
        );
        let user_destination =
            spl_associated_token_account::get_associated_token_address(&owner, &other_token_mint);

        let params = MeteoraSwapParams {
            pool: target_instruction.accounts[idx::POOL].pubkey,
            direction,
            user_source,
            user_destination,
            a_vault: target_instruction.accounts[idx::A_VAULT].pubkey,
            b_vault: target_instruction.accounts[idx::B_VAULT].pubkey,
            a_token_vault: target_instruction.accounts[idx::A_TOKEN_VAULT].pubkey,
            b_token_vault: target_instruction.accounts[idx::B_TOKEN_VAULT].pubkey,
            a_vault_lp_mint: target_instruction.accounts[idx::A_VAULT_LP_MINT].pubkey,
            b_vault_lp_mint: target_instruction.accounts[idx::B_VAULT_LP_MINT].pubkey,
            a_vault_lp: target_instruction.accounts[idx::A_VAULT_LP].pubkey,
            b_vault_lp: target_instruction.accounts[idx::B_VAULT_LP].pubkey,
            protocol_token_fee: protocol_fee_acc,
            vault_program: target_instruction.accounts[idx::VAULT_PROGRAM].pubkey,
            token_program: target_instruction.accounts[idx::TOKEN_PROGRAM].pubkey,
            mint_target_token: other_token_mint,
        };

        let recent_blockhash: Hash = *transaction.message.recent_blockhash();
        self.is_buy = true;

        self.bench
            .clone()
            .send_buy_tx_meteora(recent_blockhash, params)
            .await;

        Ok(())
    }
}
