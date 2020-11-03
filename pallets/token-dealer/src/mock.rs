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
use pallet_balances;
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
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

impl assets::Trait for Test {
    type Event = ();
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

impl pallet_balances::Trait for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = TestEvent;
    type ExistentialDeposit = Self::ExistentialDeposit;
    type AccountStore = system::Module<Test>;
    type WeightInfo = ();
}

impl Trait for Test {
    type UpwardMessageSender = MessageBrokerMock;
    type UpwardMessage = TestUpwardMessage;
    type XCMPMessageSender = MessageBrokerMock;
    type Event = TestEvent;
    type Currency = Balances;
}

mod token_dealer {
    pub use crate::Event;
}

use frame_system as system;
impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        token_dealer<T>,
        cumulus_message_broker<T>,
    }
}

pub type Assets = assets::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type TokenDealer = Module<Test>;
pub type System = frame_system::Module<Test>;

pub struct ExtBuilder {
    //spending_to_relay_rate: u128,
    //generic_to_spending_rate: u128,
    existential_deposit: u128,
    account_balances: Vec<(AccountId, Balance)>,
}

// Returns default values for genesis config
impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            // spending_to_relay_rate: 1000,
            // generic_to_spending_rate: 1,
            existential_deposit: 100,
            account_balances: vec![],
        }
    }
}

impl ExtBuilder {
    // Sets the exchange rates between assets to relay chain
    //  pub fn assets_relay_rates(mut self, rate: (u128, u128)) -> Self {
    //      self.spending_to_relay_rate = rate.0;
    //      self.generic_to_spending_rate = rate.1;
    //      self
    //  }
    pub fn existential_deposit(mut self, existential_deposit: u128) -> Self {
        self.existential_deposit = existential_deposit;
        self
    }
    pub fn free_balance(mut self, ab: Vec<(AccountId, Balance)>) -> Self {
        self.account_balances = ab;
        self
    }
    pub fn build(self) -> sp_io::TestExternalities {
        self.set_associated_consts();
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: self.account_balances,
        }
        .assimilate_storage(&mut t)
        .unwrap();
    }
}
