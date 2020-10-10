// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus. If not, see <http://www.gnu.org/licenses/>.

//! Upward messages types and traits for Polkadot, Kusama, Rococo and Westend.
//!
//! As Cumulus needs to suits multiple Polkadot-like runtimes the upward message
//! type is different for each of them. To support all of them, Cumulus provides
//! traits to write upward message generic code.

use polkadot_parachain::primitives::Id as ParaId;
use sp_std::vec::Vec;

use polkadot_core_primitives::{AccountId, Balance};
use rococo_runtime::{BalancesCall, ParachainsCall};
// mod kusama;
// mod polkadot;
// mod westend;

/// A `Balances` related upward message.
pub trait BalancesMessage<AccountId, Balance>: Sized {
    /// Transfer the given `amount` from the parachain account to the given
    /// `dest` account.
    fn transfer(dest: AccountId, amount: Balance) -> Self;
}

/// A `XCMP` related upward message.
pub trait XCMPMessage: Sized {
    /// Send the given XCMP message to given parachain.
    fn send_message(dest: ParaId, msg: Vec<u8>) -> Self;
}

/// The Rococo upward message.
pub type RococoUpwardMessage = rococo_runtime::Call;

impl BalancesMessage<AccountId, Balance> for RococoUpwardMessage {
    fn transfer(dest: AccountId, amount: Balance) -> Self {
        BalancesCall::transfer(dest, amount).into()
    }
}

impl XCMPMessage for RococoUpwardMessage {
    fn send_message(dest: ParaId, msg: Vec<u8>) -> Self {
        ParachainsCall::send_xcmp_message(dest, msg).into()
    }
}
