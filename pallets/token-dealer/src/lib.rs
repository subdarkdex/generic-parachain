#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use cumulus_primitives::{
    relay_chain::DownwardMessage,
    xcmp::{XCMPMessageHandler, XCMPMessageSender},
    DownwardMessageHandler, ParaId, UpwardMessageOrigin, UpwardMessageSender,
};
use frame_support::{
    decl_event, decl_module,
    dispatch::DispatchResult,
    traits::{Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;
use pallet_assets as assets;
use polkadot_parachain::primitives::AccountIdConversion;

// upward_message here is the same for cumulus rococo V1
// included here in preparation for Cumulus V1
pub mod upward_messages;
pub use crate::upward_messages::BalancesMessage;

mod mock;
mod tests;

/// type, id of an asset from pallet-assets
pub type AssetIdOf<T> = <T as assets::Trait>::AssetId;

/// type, balances representation for both assets and currency
pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

/// Unique identifier for the Relay Chain account
#[derive(Clone, Copy, Decode, Default, Encode, Eq, Hash, PartialEq)]
pub struct RelayAccount();

/// structure to help with Relay Chain Account setup
/// reference: polkadot/parachains/src/primitives.rs
struct TrailingZeroInput<'a>(&'a [u8]);
impl<'a> codec::Input for TrailingZeroInput<'a> {
    fn remaining_len(&mut self) -> Result<Option<usize>, codec::Error> {
        Ok(None)
    }

    fn read(&mut self, into: &mut [u8]) -> Result<(), codec::Error> {
        let len = into.len().min(self.0.len());
        into[..len].copy_from_slice(&self.0[..len]);
        for i in &mut into[len..] {
            *i = 0;
        }
        self.0 = &self.0[len..];
        Ok(())
    }
}

/// Format is b"Relay" ++ 00...; zeroes to fill AccountId.
impl<T: Encode + Decode> AccountIdConversion<T> for RelayAccount {
    fn into_account(&self) -> T {
        (b"Relay")
            .using_encoded(|b| T::decode(&mut TrailingZeroInput(b)))
            .unwrap()
    }

    fn try_from_account(x: &T) -> Option<Self> {
        x.using_encoded(|d| {
            if &d[0..5] != b"Relay" {
                return None;
            }
            let mut cursor = &d[5..];
            let result = Decode::decode(&mut cursor).ok()?;
            if cursor.iter().all(|x| *x == 0) {
                Some(result)
            } else {
                None
            }
        })
    }
}

/// Represent XCMP Message between parachains
#[derive(Encode, Decode)]
pub enum XCMPMessage<XAccountId, XBalance, XAssetIdOf> {
    /// Transfer tokens to the given account from the Parachain account.
    TransferToken(XAccountId, XBalance, Option<XAssetIdOf>),
}

/// Configuration trait of this pallet.
pub trait Trait: frame_system::Trait + assets::Trait {
    /// Event type used by the runtime.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The sender of upward messages.
    type UpwardMessageSender: UpwardMessageSender<Self::UpwardMessage>;

    /// The upward message type used by the Parachain runtime.
    type UpwardMessage: codec::Codec + BalancesMessage<Self::AccountId, BalanceOf<Self>>;

    /// The sender of XCMP messages.
    type XCMPMessageSender: XCMPMessageSender<
        XCMPMessage<Self::AccountId, BalanceOf<Self>, AssetIdOf<Self>>,
    >;
    /// The currency type that uses Currency Trait in frame_support
    type Currency: Currency<Self::AccountId>;
}

decl_event! {
    pub enum Event<T> where
        AssetId = AssetIdOf<T>,
        AccountId = <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>
    {
        /// Transferred tokens to the account on the relay chain.
        /// (sender_accont_local, asset_id_local, reciever_account_on_relay_chain, transfer_amount)
        TransferredTokensToRelayChain(AccountId, Option<AssetId>, AccountId, Balance),
        /// Transferred tokens to the account on the parachain.
        /// (sender_account_local, asset_id_local, para_id_dest, reciever_account_dest, asset_id_dest, transfer_amount,)
        TransferredTokensToParachain(AccountId, Option<AssetId>, ParaId, AccountId, Option<AssetId>, Balance),
        /// Transferred tokens to the account on request from the relay chain.
        /// (reciever_account_local, amount, Option<AssetId>, result)
        TransferredTokensFromRelayChain(AccountId, Balance, Option<AssetId>, DispatchResult),
        /// Transferred tokens to the account on request from parachain.
        /// (ParaId, reciever_account_on_para, amount, assetId, result)
        TransferredTokensViaXCMP(ParaId, AccountId, Balance, Option<AssetId>, DispatchResult),
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Transfer `amount` of tokens (local asset_id) or Currency from Parachain account to the Relay Chain
        /// at the given `dest` account.
        #[weight = 10]
        pub fn transfer_tokens_to_relay_chain(origin, dest: T::AccountId, amount: BalanceOf<T>, asset_id: Option<AssetIdOf<T>>) {
            let who = ensure_signed(origin.clone())?;
            Self::make_transfer_to_relay_chain(&asset_id, &who, &dest, amount)?;
            Self::deposit_event(Event::<T>::TransferredTokensToRelayChain(who, asset_id, dest, amount));
        }

        /// Transfer `amount` of tokens (local asset_id) or Currency to another parachain at the
        /// give `dest` account. If the other parachain has assets and needs to be mapped, use the
        /// Module fn `make_transfer_to_parachain` instead.
        #[weight = 10]
        pub fn transfer_assets_to_parachain_chain(
            origin,
            para_id: u32,
            dest: T::AccountId,
            amount: BalanceOf<T>,
            asset_id: Option<AssetIdOf<T>>,
        ) {
            let who = ensure_signed(origin.clone())?;

            let para_id: ParaId = para_id.into();
            Self::make_transfer_to_parachain(&who, &asset_id, para_id, &dest, &asset_id, amount)?;
            Self::deposit_event(Event::<T>::TransferredTokensToParachain(who, asset_id, para_id, dest, asset_id, amount ));
        }

        fn deposit_event() = default;
    }
}

/// This is a hack to convert from one generic type to another where we are sure that both are the
/// same type/use the same encoding.
fn convert_hack<O: Decode>(input: &impl Encode) -> O {
    input.using_encoded(|e| Decode::decode(&mut &e[..]).expect("Must be compatible; qed"))
}

impl<T: Trait> Module<T> {
    /// Transfer Asset(asset_id == Some(id)) or Currency (asset_id == None) to the Relay Chain;
    /// This transfers Asset/Currency from this Parachain's account to the RelayAccount on this
    /// parachain and sends an upward message to the relay chain
    ///
    /// WARN: Must ensure parachain account on relay chain has enough balance to transfer out
    /// from, this does not guarentee that Relay Chain `dest` account is credited.
    pub fn make_transfer_to_relay_chain(
        asset_id: &Option<AssetIdOf<T>>,
        from: &T::AccountId,
        dest: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let relay_account: T::AccountId = RelayAccount::default().into_account();

        if let Some(id) = asset_id {
            let amount = convert_hack(&amount);
            <assets::Module<T>>::make_transfer(from, *id, &relay_account, amount)?;
        } else {
            // Transfer parachain asset to the relay_account (which is on this parachain)
            T::Currency::transfer(
                from,
                &relay_account,
                amount,
                ExistenceRequirement::KeepAlive,
            )?;
        }

        // Send upward message to Relay Chain to transfer `amount` from this parachain's
        // account on the relay chain to dest account.
        let msg = <T::UpwardMessage>::transfer(dest.clone(), amount.clone());
        <T as Trait>::UpwardMessageSender::send_upward_message(&msg, UpwardMessageOrigin::Signed)
            .expect("Should not fail; qed");
        Ok(())
    }

    /// Transfer Asset(asset_id == Some(id)) or Currency (asset_id == None) to a dest Parachain at
    /// para_id;
    /// This transfers Asset/Currency from this Parachain's account to the dest Parachain's
    /// account, derived from their para_id, on this parachain and sends an XCMP message to dest
    /// parachain
    /// INFO: If the other parachain has assets, use `dest_asset_id` to inform other parachain
    /// which asset_id to complete the transfer.
    ///
    /// WARN: Must ensure that this parachain account on the dest parachain has enough balance to transfer out
    /// from, this function does not guarentee that dest parachain `dest` account is credited.
    pub fn make_transfer_to_parachain(
        from: &T::AccountId,
        asset_id: &Option<AssetIdOf<T>>,
        para_id: ParaId,
        dest: &T::AccountId,
        dest_asset_id: &Option<AssetIdOf<T>>,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let para_account: T::AccountId = para_id.into_account();

        if let Some(id) = asset_id {
            <assets::Module<T>>::make_transfer(from, *id, &para_account, convert_hack(&amount))?;
        } else {
            T::Currency::transfer(from, &para_account, amount, ExistenceRequirement::KeepAlive)?;
        }

        // Send XCMPMessage to the other parachain
        T::XCMPMessageSender::send_xcmp_message(
            para_id.into(),
            &XCMPMessage::TransferToken(dest.clone(), amount, *dest_asset_id),
        )
        .expect("Should not fail; qed");
        Ok(())
    }
}

impl<T: Trait> DownwardMessageHandler for Module<T> {
    /// Handles messages from the Relay Chain, only match to `TransferInto` type
    /// Here we use the remark field of the downward message to decode into Option<AssetId>
    fn handle_downward_message(msg: &DownwardMessage) {
        match msg {
            DownwardMessage::TransferInto(dest, relay_amount, remark) => {
                let dest: T::AccountId = convert_hack(&dest);
                let relay_amount: BalanceOf<T> = convert_hack(relay_amount);
                let relay_account = RelayAccount::default().into_account();
                // remark has a concerte type [u8; 32]
                let asset_id: Option<AssetIdOf<T>> = convert_hack(remark);

                let res = match asset_id {
                    Some(id) => <assets::Module<T>>::make_transfer(
                        &relay_account,
                        id,
                        &dest,
                        convert_hack(&relay_amount.clone()),
                    ),
                    None => T::Currency::transfer(
                        &relay_account,
                        &dest,
                        relay_amount,
                        ExistenceRequirement::KeepAlive,
                    ),
                };

                Self::deposit_event(Event::<T>::TransferredTokensFromRelayChain(
                    dest.clone(),
                    relay_amount,
                    asset_id,
                    res,
                ));
            }
            _ => {}
        }
    }
}

impl<T: Trait> XCMPMessageHandler<XCMPMessage<T::AccountId, BalanceOf<T>, AssetIdOf<T>>>
    for Module<T>
{
    /// Handles messages from other parachains, only match to `TransferToken` message
    fn handle_xcmp_message(
        src: ParaId,
        msg: &XCMPMessage<T::AccountId, BalanceOf<T>, AssetIdOf<T>>,
    ) {
        match msg {
            XCMPMessage::TransferToken(dest, amount, asset_id) => {
                let para_account = src.clone().into_account();

                let res = match asset_id {
                    Some(id) => <assets::Module<T>>::make_transfer(
                        &para_account,
                        *id,
                        &dest,
                        convert_hack(&amount.clone()),
                    ),
                    None => T::Currency::transfer(
                        &para_account,
                        &dest,
                        *amount,
                        ExistenceRequirement::KeepAlive,
                    ),
                };

                Self::deposit_event(Event::<T>::TransferredTokensViaXCMP(
                    src,
                    dest.clone(),
                    amount.clone(),
                    *asset_id,
                    res,
                ));
            }
        }
    }
}
