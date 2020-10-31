// Copyright 2019-2020
//     by  Centrality Investments Ltd.
//     and Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Mocks for the module.

#![cfg(test)]

pub use super::*;
use cumulus_message_broker;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use parachain_info;
use polkadot_core_primitives::AccountId as AccountId32;
use sp_core::H256;
use sp_io;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}
use upward_messages;

type Balance = u128;
type AccountId = AccountId32;
type AssetId = u32;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = ();
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

impl generic_asset::Trait for Test {
    type Event = TestEvent;
    type Balance = Balance;
    type AssetId = AssetId;
}

#[derive(Encode, Decode)]
pub struct TestUpwardMessage {}
impl upward_messages::BalancesMessage<AccountId, Balance> for TestUpwardMessage {
    fn transfer(_a: AccountId, _b: Balance) -> Self {
        TestUpwardMessage {}
    }
}

impl upward_messages::XCMPMessage for TestUpwardMessage {
    fn send_message(_dest: ParaId, _msg: Vec<u8>) -> Self {
        TestUpwardMessage {}
    }
}

pub struct MessageBrokerMock {}
impl UpwardMessageSender<TestUpwardMessage> for MessageBrokerMock {
    fn send_upward_message(
        _msg: &TestUpwardMessage,
        _origin: UpwardMessageOrigin,
    ) -> Result<(), ()> {
        Ok(())
    }
}

impl XCMPMessageSender<XCMPMessage<AccountId, Balance, AssetId>> for MessageBrokerMock {
    fn send_xcmp_message(
        _dest: ParaId,
        _msg: &XCMPMessage<AccountId, Balance, AssetId>,
    ) -> Result<(), ()> {
        Ok(())
    }
}

impl parachain_info::Trait for Test {}

impl Trait for Test {
    type UpwardMessageSender = MessageBrokerMock;
    type UpwardMessage = TestUpwardMessage;
    type XCMPMessageSender = MessageBrokerMock;
    type Event = TestEvent;
}

mod token_dealer {
    pub use crate::Event;
}

use frame_system as system;
impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        token_dealer<T>,
        generic_asset<T>,
        cumulus_message_broker<T>,
    }
}

pub type GenericAsset = generic_asset::Module<Test>;
pub type TokenDealer = Module<Test>;
pub type System = frame_system::Module<Test>;

pub struct ExtBuilder {
    spending_to_relay_rate: u128,
    generic_to_spending_rate: u128,
    asset_id: u32,
    next_asset_id: u32,
    accounts: Vec<AccountId>,
    initial_balance: u128,
}

// Returns default values for genesis config
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            spending_to_relay_rate: 1000,
            generic_to_spending_rate: 1,
            asset_id: 0,
            next_asset_id: 1000,
            accounts: vec![],
            initial_balance: 0,
        }
    }
}

impl ExtBuilder {
    // Sets free balance to genesis config
    pub fn free_balance(mut self, free_balance: (u32, AccountId, u128)) -> Self {
        self.asset_id = free_balance.0;
        self.accounts = vec![free_balance.1];
        self.initial_balance = free_balance.2;
        self
    }

    // Sets the exchange rates between assets to relay chain
    pub fn assets_relay_rates(mut self, rate: (u128, u128)) -> Self {
        self.spending_to_relay_rate = rate.0;
        self.generic_to_spending_rate = rate.1;
        self
    }

    // builds genesis config -- add to build GenericAsset too
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let generic_asset_genesis = generic_asset::GenesisConfig::<Test> {
            assets: vec![self.asset_id],
            endowed_accounts: self.accounts,
            initial_balance: self.initial_balance,
            next_asset_id: self.next_asset_id,
            staking_asset_id: 0,
            spending_asset_id: 0,
        };
        let genesis = GenesisConfig::<Test> {
            spending_to_relay_rate: self.spending_to_relay_rate,
            generic_to_spending_rate: self.generic_to_spending_rate,
        };

        generic_asset_genesis.assimilate_storage(&mut t).unwrap();
        genesis.assimilate_storage(&mut t).unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
