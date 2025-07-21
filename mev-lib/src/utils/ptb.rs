use crate::{constant, service::db_service};
use db::repositories::{CoinRepository, PoolRepository};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fastcrypto::{ed25519::Ed25519KeyPair, hash::HashFunction};
use rust_decimal::{prelude::*, Decimal};
use shared_crypto::intent::{Intent, IntentMessage};
use std::{
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
};
use sui_json_rpc_types::SuiTransactionBlockResponse;
use sui_sdk::{
    rpc_types::{
        self, SuiExecutionStatus, SuiObjectData, SuiObjectDataFilter, SuiObjectDataOptions,
        SuiObjectResponseQuery, SuiTransactionBlockResponseOptions,
    },
    types::{
        self,
        programmable_transaction_builder::ProgrammableTransactionBuilder,
        quorum_driver_types::ExecuteTransactionRequestType,
        signature::{self, GenericSignature},
        transaction::{self, Argument, Command, ObjectArg, Transaction, TransactionData},
        type_input::TypeInput,
        Identifier, TypeTag,
    },
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, SequenceNumber, SuiAddress},
    crypto::{
        get_key_pair_from_rng, DefaultHash, EncodeDecodeBase64, Signer, SuiKeyPair, SuiSignature,
    },
    digests::TransactionDigest,
    transaction::ProgrammableTransaction,
};
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn, Level};

pub struct PTBHelper {
    pub client: Arc<SuiClient>,
    pub shio_client: Arc<SuiClient>,
    pub db_pool_service: Arc<db_service::pool::PoolService>,
    pub db_lending_service: Arc<db_service::lending::LendingService>,
}

impl PTBHelper {
    pub fn new(
        client: Arc<SuiClient>,
        shio_client: Arc<SuiClient>,
        db_pool_service: Arc<db_service::pool::PoolService>,
        db_lending_service: Arc<db_service::lending::LendingService>,
    ) -> Self {
        PTBHelper {
            client,
            shio_client,
            db_pool_service,
            db_lending_service,
        }
    }

    /// Find the gas coin for a programmable transaction builder (PTB) given a sender address.
    /// The gas coin is the SUI coin with the highest balance available for the sender.
    ///
    pub async fn find_gas_coin_for_ptb(&self, sender: &str) -> Result<rpc_types::Coin> {
        let sender_address = SuiAddress::from_str(sender)?;
        let sui_coins = self
            .get_all_coins_by_address_and_type(&sender_address, constant::SUI_COIN)
            .await?;

        if sui_coins.is_empty() {
            return Err(anyhow!(
                "No SUI coin found for the sender address {}",
                sender
            ));
        }

        let gas_coin = sui_coins
            .iter()
            .max_by_key(|coin| coin.balance)
            .ok_or_else(|| anyhow!("No coins found for input"))?;

        Ok(gas_coin.clone())
    }

    pub async fn get_coins_for_amount(
        &self,
        address: &SuiAddress,
        coin_type: &str,
        amount: Decimal,
    ) -> Result<Vec<rpc_types::Coin>> {
        let coins = self
            .get_all_coins_by_address_and_type(address, coin_type)
            .await?;

        let mut results = Vec::new();
        let mut total_amount = Decimal::ZERO;

        for coin in coins {
            if total_amount >= amount {
                break;
            }

            if coin.balance == 0 {
                continue;
            }

            results.push(coin.clone());
            total_amount += Decimal::from(coin.balance);
        }

        if total_amount < amount {
            return Err(anyhow!(
                "Insufficient balance. Required: {}, Available: {}",
                amount,
                total_amount
            ));
        }

        Ok(results)
    }

    pub async fn get_all_coins_by_address_and_type(
        &self,
        address: &SuiAddress,
        coin_type: &str,
    ) -> Result<Vec<rpc_types::Coin>> {
        let mut results = Vec::new();
        let count = 50;
        let mut next_cursor = None;

        loop {
            let coins = self
                .client
                .coin_read_api()
                .get_coins(
                    *address,
                    Some(coin_type.to_string()),
                    next_cursor,
                    Some(count),
                )
                .await?;

            results.extend(coins.data);

            if coins.has_next_page {
                // If there are more pages, continue fetching
                next_cursor = coins.next_cursor.clone();
            } else {
                // No more pages, break the loop
                break;
            }
        }

        Ok(results)
    }

    /// Build a Clock object for PTB
    pub async fn build_clock_arg(&self, mutable: bool) -> Result<ObjectArg> {
        self.build_shared_obj_arg(constant::CLOCK_OBJECT_ID, mutable)
            .await
    }

    /// Build a Shared object for PTB
    pub async fn build_shared_obj_arg(&self, object_id: &str, mutable: bool) -> Result<ObjectArg> {
        match self.db_lending_service.find_shared_object_by_id(object_id) {
            Ok(shared_object) => {
                info!(
                    "Found shared object {} in database, use the cached version",
                    object_id
                );

                // If the shared object is found in the database, return it
                Ok(ObjectArg::SharedObject {
                    id: ObjectID::from_hex_literal(object_id)?,
                    initial_shared_version: SequenceNumber::from_u64(
                        shared_object.initial_shared_version as u64,
                    ),
                    mutable,
                })
            }
            Err(e) => {
                info!(
                    "Shared object {} is not found in database, fetching it from Sui",
                    object_id
                );

                // If the shared object is not found in the database, fetch it from Sui
                let object_data_options = SuiObjectDataOptions::full_content();

                let sui_object_id = ObjectID::from_hex_literal(object_id)?;

                let obj_response = self
                    .client
                    .read_api()
                    .get_object_with_options(sui_object_id, object_data_options.clone())
                    .await?;

                let obj_data = obj_response
                    .data
                    .as_ref()
                    .ok_or_else(|| anyhow!("Failed to get object data"))?;

                let initial_shared_version = obj_data
                    .owner
                    .as_ref()
                    .ok_or_else(|| anyhow!("Failed to get object owner for clock"))?
                    .start_version()
                    .ok_or_else(|| anyhow!("Failed to get start version for clock"))?;

                // Cache the shared object to the database
                if let Err(e) = self
                    .db_lending_service
                    .save_shared_object_to_db(object_id, initial_shared_version.into())
                {
                    return Err(anyhow!(
                        "Failed to save shared object {} to database: {}",
                        object_id,
                        e
                    ))?;
                }

                Ok(ObjectArg::SharedObject {
                    id: sui_object_id,
                    initial_shared_version,
                    mutable,
                })
            }
        }
    }

    /// Build a Owned object for PTB
    pub async fn build_owned_obj_arg(
        &self,
        owner_address: SuiAddress,
        object_type: &str,
    ) -> Result<ObjectArg> {
        let owned_objects = self
            .find_owned_objects_given_owner_address_and_type(owner_address, object_type, true)
            .await?;
        if owned_objects.is_empty() {
            return Err(anyhow!(
                "No owned objects found for address {} and type {}",
                owner_address,
                object_type
            ));
        }

        let first_object = &owned_objects[0];
        Ok(ObjectArg::ImmOrOwnedObject((
            first_object.object_id,
            first_object.version,
            first_object.digest,
        )))
    }

    pub fn create_zero_coin_command(&self, coin_type: &str) -> Result<Command> {
        let coin_type_arg = TypeTag::from_str(coin_type)?;

        Ok(Command::move_call(
            ObjectID::from_hex_literal("0x2")?,
            Identifier::new("coin")?,
            Identifier::new("zero")?,
            vec![coin_type_arg],
            vec![],
        ))
    }

    pub fn destroy_zero_balance_command(
        &self,
        coin_type: &str,
        balance_arg: Argument,
    ) -> Result<Command> {
        let coin_type_arg = TypeTag::from_str(coin_type)?;

        Ok(Command::move_call(
            ObjectID::from_hex_literal("0x2")?,
            Identifier::new("balance")?,
            Identifier::new("destroy_zero")?,
            vec![coin_type_arg],
            vec![balance_arg],
        ))
    }

    pub fn coin_into_balance_command(
        &self,
        coin_type: &str,
        coin_arg: Argument,
    ) -> Result<Command> {
        let coin_type_arg = TypeTag::from_str(coin_type)?;

        Ok(Command::move_call(
            ObjectID::from_hex_literal("0x2")?,
            Identifier::new("coin")?,
            Identifier::new("into_balance")?,
            vec![coin_type_arg],
            vec![coin_arg],
        ))
    }

    pub fn coin_from_balance_command(
        &self,
        coin_type: &str,
        balance_arg: Argument,
    ) -> Result<Command> {
        let coin_type_arg = TypeTag::from_str(coin_type)?;

        Ok(Command::move_call(
            ObjectID::from_hex_literal("0x2")?,
            Identifier::new("coin")?,
            Identifier::new("from_balance")?,
            vec![coin_type_arg],
            vec![balance_arg],
        ))
    }

    /// Create the input coin for a command in a programmable transaction builder.
    /// Given a sender address, coin type, amount, gas budget, and gas coin,
    /// this function retrieves the necessary coins, splits or merges them as needed,
    /// and returns the argument for the input coin along with the updated command index.
    ///
    /// Returns a tuple (input_coin_arg, updated_command_index).
    ///
    pub async fn create_coin_input_for_ptb(
        &self,
        ptb: &mut ProgrammableTransactionBuilder,
        sender: &str,
        coin_type: &str,
        amount_in: u64,
        gas_budget: u64,
        gas_coin: rpc_types::Coin,
        command_index: u16,
    ) -> Result<(Argument, u16)> {
        let sender_address = SuiAddress::from_str(sender)?;
        let coin_in_sui = coin_type == constant::SUI_COIN;
        let mut command_index = command_index;

        let coins_in = self
            .get_coins_for_amount(
                &sender_address,
                coin_type,
                Decimal::from_u64(amount_in)
                    .ok_or_else(|| anyhow!("Failed to convert amount_in to Decimal"))?,
            )
            .await?;

        if coins_in.is_empty() {
            return Err(anyhow!(
                "No coins found for the address {} and coin type {}",
                sender,
                coin_type,
            ));
        }

        let coin_input_arg = if coin_in_sui {
            // split input coin from the gas coin

            let mut split_amount = gas_coin.balance as i64 - gas_budget as i64;
            if split_amount <= 0 {
                split_amount = 0;
            } else if split_amount > amount_in as i64 {
                split_amount = amount_in as i64;
            }

            info!("Split amount: {}", split_amount);

            let split_amount_arg = ptb.pure::<u64>(split_amount as u64)?;

            ptb.command(Command::SplitCoins(
                Argument::GasCoin,
                vec![split_amount_arg],
            ));
            command_index += 1;

            let coin_input_arg = Argument::Result(command_index - 1); // the result of the split command

            // merge splited coin to the remaining coins to create a single input coin
            let other_coins_arg = coins_in
                .iter()
                .filter(|coin| coin.coin_object_id != gas_coin.coin_object_id)
                .map(|coin| {
                    ptb.obj(ObjectArg::ImmOrOwnedObject((
                        coin.coin_object_id,
                        coin.version,
                        coin.digest,
                    )))
                })
                .collect::<Result<Vec<Argument>>>()?;

            if !other_coins_arg.is_empty() {
                info!("Merging coins: {:?}", other_coins_arg);
                ptb.command(Command::MergeCoins(coin_input_arg, other_coins_arg));
                command_index += 1;
            }

            coin_input_arg
        } else {
            let primary_coin_arg = ptb.obj(ObjectArg::ImmOrOwnedObject((
                coins_in[0].coin_object_id,
                coins_in[0].version,
                coins_in[0].digest,
            )))?; // select first coin as primary

            // merge all other coins to the primary coin
            let other_coins_arg = coins_in
                .iter()
                .skip(1) // skip the first coin, which is already used as input
                .map(|coin| {
                    ptb.obj(ObjectArg::ImmOrOwnedObject((
                        coin.coin_object_id,
                        coin.version,
                        coin.digest,
                    )))
                })
                .collect::<Result<Vec<Argument>>>()?;

            if !other_coins_arg.is_empty() {
                info!("Merging coins: {:?}", other_coins_arg);
                ptb.command(Command::MergeCoins(primary_coin_arg, other_coins_arg));
                command_index += 1;
            }

            // split the required amount_in from the primary coin
            // and use the splited coin as input for the PTB command
            let split_amount_arg = ptb.pure::<u64>(amount_in)?;

            ptb.command(Command::SplitCoins(
                primary_coin_arg,
                vec![split_amount_arg],
            ));
            command_index += 1;

            Argument::Result(command_index - 1) // the result of the split command
        };

        Ok((coin_input_arg, command_index))
    }

    pub async fn find_owned_objects_given_owner_address_and_type(
        &self,
        owner_address: SuiAddress,
        object_type: &str,
        is_full_content: bool,
    ) -> Result<Vec<SuiObjectData>> {
        let object_data_options = if is_full_content {
            SuiObjectDataOptions::full_content()
        } else {
            SuiObjectDataOptions::default()
        };

        let query = SuiObjectResponseQuery {
            filter: Some(SuiObjectDataFilter::StructType(
                sui_types::parse_sui_struct_tag(object_type)?,
            )),
            options: Some(object_data_options),
        };

        let objects_response = self
            .client
            .read_api()
            .get_owned_objects(owner_address, Some(query), None, None)
            .await?;

        if objects_response.data.is_empty() {
            return Err(anyhow!(
                "No objects found for owner address {} and type {}",
                owner_address,
                object_type
            ));
        }

        let objects = objects_response
            .data
            .into_iter()
            .filter_map(|obj| obj.data)
            .collect::<Vec<_>>();

        Ok(objects)
    }

    /// Fetches the coin metadata for a list of coin types.
    /// This is executed in parallel to improve performance.
    ///
    pub async fn fetch_coins_metadata(
        &self,
        coin_types: Vec<String>,
    ) -> Result<Vec<crate::types::Coin>> {
        let mut coin_results = Vec::new();

        for coin_type in coin_types.iter() {
            let coin = self.get_coin_from_type(coin_type).await?;
            coin_results.push(coin);
        }

        Ok(coin_results)
    }

    /// Fetches the coin metadata for a given coin type.
    /// Firstly it checks the local database for the coin metadata.
    /// If not found, it fetches the metadata from the Sui client and stores it in the database.
    pub async fn get_coin_from_type(&self, coin_type: &str) -> Result<crate::types::Coin> {
        match self.db_pool_service.find_coin_by_type(coin_type).await {
            Ok(coin) => Ok(crate::types::Coin {
                coin_type: coin.coin_type,
                decimals: coin.decimals as u8,
                name: coin.name,
                symbol: coin.symbol,
                pyth_feed_id: coin.pyth_feed_id,
                pyth_info_object_id: coin.pyth_info_object_id,
            }),
            Err(_) => {
                if coin_type == constant::SUI_COIN {
                    return Ok(crate::types::Coin {
                        coin_type: constant::SUI_COIN.to_string(),
                        decimals: 9,
                        name: Some("Sui".to_string()),
                        symbol: Some("SUI".to_string()),
                        pyth_feed_id: None,
                        pyth_info_object_id: None,
                    });
                }

                let metadata = self
                    .client
                    .coin_read_api()
                    .get_coin_metadata(coin_type.to_string())
                    .await?
                    .ok_or_else(|| {
                        anyhow!("Failed to get coin metadata for type: {}", coin_type)
                    })?;

                Ok(crate::types::Coin {
                    coin_type: coin_type.to_string(),
                    decimals: metadata.decimals,
                    name: Some(metadata.name),
                    symbol: Some(metadata.symbol),
                    pyth_feed_id: None,
                    pyth_info_object_id: None,
                })
            }
        }
    }

    pub async fn sign_and_send_tx(
        &self,
        builder: ProgrammableTransaction,
        sender: Arc<SuiKeyPair>,
        gas_coin: sui_json_rpc_types::Coin,
        gas_budget: u64,
        gas_price: u64,
        use_shio_endpoint: bool,
    ) -> Result<SuiTransactionBlockResponse> {
        let sender_address = SuiAddress::from(&sender.public());

        let tx_data = TransactionData::new_programmable(
            sender_address,
            vec![gas_coin.object_ref()],
            builder,
            gas_budget,
            gas_price,
        );

        let intent_msg = IntentMessage::new(Intent::sui_transaction(), tx_data);
        let raw_tx = bcs::to_bytes(&intent_msg).expect("bcs should not fail");
        let mut hasher = DefaultHash::default();
        hasher.update(raw_tx.clone());
        let digest = hasher.finalize().digest;

        let signature = sender.sign(&digest);

        // submit tx
        let client = if use_shio_endpoint {
            self.shio_client.clone()
        } else {
            self.client.clone()
        };

        let tx_response = client
            .quorum_driver_api()
            .execute_transaction_block(
                transaction::Transaction::from_generic_sig_data(
                    intent_msg.value,
                    vec![signature::GenericSignature::Signature(signature)],
                ),
                SuiTransactionBlockResponseOptions::new(),
                None,
            )
            .await?;

        Ok(tx_response)
    }
}
