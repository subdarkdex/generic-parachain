// Copyright 2020 Parity Technologies (UK) Ltd.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use cumulus_primitives::{
    relay_chain::DownwardMessage,
    xcmp::{XCMPMessageHandler, XCMPMessageSender},
    DownwardMessageHandler, ParaId, UpwardMessageOrigin, UpwardMessageSender,
};
use frame_support::{decl_event, decl_module, decl_storage, dispatch::DispatchResult};
use frame_system::ensure_signed;
use pallet_generic_asset as generic_asset;
use polkadot_parachain::primitives::AccountIdConversion;

pub mod upward_messages;
pub use crate::upward_messages::BalancesMessage;

mod mock;
mod tests;

pub type AssetIdOf<T> = <T as generic_asset::Trait>::AssetId;
pub type BalanceOf<T> = <T as generic_asset::Trait>::Balance;

/// Unique identifier of a parachain.
#[derive(Clone, Copy, Decode, Default, Encode, Eq, Hash, PartialEq)]
pub struct RelayId();

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

/// Format is b"Relay" ++ 00.... where 00... is indefinite trailing
/// zeroes to fill AccountId.
impl<T: Encode + Decode + Default> AccountIdConversion<T> for RelayId {
    fn into_account(&self) -> T {
        (b"Relay", self)
            .using_encoded(|b| T::decode(&mut TrailingZeroInput(b)))
            .unwrap_or_default()
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

#[derive(Encode, Decode)]
pub enum XCMPMessage<XAccountId, XBalance, XAssetIdOf> {
    /// Transfer tokens to the given account from the Parachain account.
    TransferToken(XAccountId, XBalance),
    TransferAsset(XAccountId, XBalance, XAssetIdOf),
}

/// Configuration trait of this pallet.
pub trait Trait: frame_system::Trait + generic_asset::Trait {
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
}

decl_storage! {
    trait Store for Module<T: Trait> as TokenDealer {
        pub SpendingToRelayRate get(fn spending_to_relay_rate) config(): BalanceOf<T>;
        pub GenericToSpendingRate get(fn generic_to_spending_rate) config(): BalanceOf<T>;
    }
}

decl_event! {
    pub enum Event<T> where
        AssetId = AssetIdOf<T>,
        AccountId = <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>
    {
        /// Transferred tokens to the account on the relay chain.
        /// (Relay_chain_account_on_para, reciever_account_on_dest_chain, transfer_amount)
        TransferredTokensToRelayChain(AccountId, AccountId, Balance),
        /// Transferred tokens to the account on the relay chain.
        /// (ParaId, parachain_account_on_para, reciever_account_on_dest_chain, transfer_amount,
        /// AssetId)
        TransferredTokensToParachain(ParaId, AccountId, AccountId, Balance, AssetId),
        /// Transferred tokens to the account on request from the relay chain.
        /// (reciever_account_on_para, amount, result)
        TransferredTokensFromRelayChain(AccountId, Balance, DispatchResult),
        /// Transferred tokens to the account from the given parachain account.
        /// (ParaId, reciever_account_on_para, amount, assetId, result )
        TransferredTokensViaXCMP(ParaId, AccountId, Balance, AssetId, DispatchResult),
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Transfer `amount` of tokens on the relay chain from the Parachain account to
        /// the given `dest` account.
        #[weight = 10]
        fn transfer_tokens_to_relay_chain(origin, dest: T::AccountId, amount: BalanceOf<T>, asset_id: AssetIdOf<T>) {
            let who = ensure_signed(origin)?;

            let relay_id = RelayId::default();
            let relay_account = relay_id.into_account();

            <generic_asset::Module<T>>::make_transfer(
                &asset_id, &who, &relay_account, amount
            )?;

            // Not tested because need network integration test
            let mut relay_amount = amount / Self::spending_to_relay_rate();
            if asset_id != <generic_asset::Module<T>>::spending_asset_id() {
                relay_amount /= Self::generic_to_spending_rate();
            }

            let msg = <T::UpwardMessage>::transfer(dest.clone(), amount.clone());
            <T as Trait>::UpwardMessageSender::send_upward_message(&msg, UpwardMessageOrigin::Signed)
                .expect("Should not fail; qed");

            Self::deposit_event(Event::<T>::TransferredTokensToRelayChain(relay_account, dest, amount));
        }

        /// Transfer `amount` of tokens to another parachain.
        #[weight = 10]
        fn transfer_assets_to_parachain_chain(
            origin,
            para_id: u32,
            dest: T::AccountId,
            amount: BalanceOf<T>,
            asset_id: AssetIdOf<T>,
        ) {
            //TODO we don't make sure that the parachain has some tokens on the other parachain.
            let who = ensure_signed(origin)?;

            let para_id: ParaId = para_id.into();
            let para_account = para_id.into_account();

            <generic_asset::Module<T>>::make_transfer(
                &asset_id, &who, &para_account, amount
            )?;

            T::XCMPMessageSender::send_xcmp_message(
                para_id.into(),
                &XCMPMessage::TransferAsset(dest.clone(), amount, asset_id),
            ).expect("Should not fail; qed");

            Self::deposit_event(Event::<T>::TransferredTokensToParachain(para_id, para_account, dest, amount, asset_id));
        }

        fn deposit_event() = default;
    }
}

/// This is a hack to convert from one generic type to another where we are sure that both are the
/// same type/use the same encoding.
fn convert_hack<O: Decode>(input: &impl Encode) -> O {
    input.using_encoded(|e| Decode::decode(&mut &e[..]).expect("Must be compatible; qed"))
}

impl<T: Trait> DownwardMessageHandler for Module<T> {
    fn handle_downward_message(msg: &DownwardMessage) {
        match msg {
            DownwardMessage::TransferInto(dest, relay_amount, remark) => {
                let dest: T::AccountId = convert_hack(&dest);
                let relay_amount: BalanceOf<T> = convert_hack(relay_amount);
                let relay_account = RelayId::default().into_account();
                // remark has a concerte type [u8; 32]
                let asset_id: AssetIdOf<T> = convert_hack(remark);

                let mut amount = relay_amount * Self::spending_to_relay_rate();
                if asset_id != <generic_asset::Module<T>>::spending_asset_id() {
                    amount = amount * Self::generic_to_spending_rate();
                }
                let res = <generic_asset::Module<T>>::make_transfer(
                    &asset_id,
                    &relay_account,
                    &dest,
                    amount.clone(),
                );

                Self::deposit_event(Event::<T>::TransferredTokensFromRelayChain(
                    dest.clone(),
                    amount,
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
    fn handle_xcmp_message(
        src: ParaId,
        msg: &XCMPMessage<T::AccountId, BalanceOf<T>, AssetIdOf<T>>,
    ) {
        match msg {
            XCMPMessage::TransferAsset(dest, amount, asset_id) => {
                let para_account = src.clone().into_account();

                // asset_id cast into generic asset transfer
                let res = <generic_asset::Module<T>>::make_transfer(
                    &asset_id,
                    &para_account,
                    dest,
                    amount.clone(),
                );

                Self::deposit_event(Event::<T>::TransferredTokensViaXCMP(
                    src,
                    dest.clone(),
                    amount.clone(),
                    asset_id.clone(),
                    res,
                ));
            }
            XCMPMessage::TransferToken(dest, amount) => {
                let para_account = src.clone().into_account();

                // asset_id cast into generic asset transfer
                let res = <generic_asset::Module<T>>::make_transfer(
                    &<generic_asset::Module<T>>::spending_asset_id(),
                    &para_account,
                    dest,
                    amount.clone(),
                );

                Self::deposit_event(Event::<T>::TransferredTokensViaXCMP(
                    src,
                    dest.clone(),
                    amount.clone(),
                    <generic_asset::Module<T>>::spending_asset_id(),
                    res,
                ));
            }
        }
    }
}
