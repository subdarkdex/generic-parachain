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

//! Tests for the module.

#![cfg(test)]

use super::*;
use crate::mock::{
    new_test_ext, ExtBuilder, GenericAsset, Origin, System, Test, TestEvent, TokenDealer,
};
use frame_support::{assert_noop, assert_ok};

#[test]
fn transfer_to_relay_chain_deducts_parachain_account() {
    // 1 DOT = 1000 spending asset, 1 spending asset = 1 generic asset
    let assets_relay_rates = (1000, 1);
    let initial_para_amount = 10000;
    let transfer_para_amount = 1000;
    let from = [0u8; 32];
    let relay_account: [u8; 32] = RelayId::default().into_account();
    let expected_balance = initial_para_amount - transfer_para_amount;

    ExtBuilder::default()
        // assetid, account, balance
        .free_balance((0, from.into(), initial_para_amount))
        // spending to relay, generic to spending
        .assets_relay_rates(assets_relay_rates)
        .build()
        .execute_with(|| {
            assert_ok!(TokenDealer::transfer_tokens_to_relay_chain(
                Origin::signed(from.into()),
                from.into(),
                transfer_para_amount,
                0
            ));
            assert_eq!(
                GenericAsset::free_balance(&0, &from.into()),
                expected_balance
            );
            assert_eq!(
                GenericAsset::free_balance(&0, &relay_account.into()),
                transfer_para_amount
            );
        });
}

#[test]
fn handles_downward_messages() {
    // 1 DOT = 1000 spending asset, 1 spending asset = 1 generic asset
    let assets_relay_rates = (1000, 1);
    let initial_relay_account_ammount = 10000;
    let transfer_relay_amount = 9;
    let expected_balance = transfer_relay_amount * assets_relay_rates.0;

    let dest = [0u8; 32];
    let remark = [0u8; 32];
    let relay_account: [u8; 32] = RelayId::default().into_account();
    let downward_message =
        DownwardMessage::TransferInto(dest.into(), transfer_relay_amount, remark);

    ExtBuilder::default()
        // assetid, account, balance
        .free_balance((0, relay_account.into(), initial_relay_account_ammount))
        .assets_relay_rates((1000, 1))
        // spending to relay, generic to spending
        .build()
        .execute_with(|| {
            TokenDealer::handle_downward_message(&downward_message);
            assert_eq!(
                GenericAsset::free_balance(&0, &relay_account.into()),
                initial_relay_account_ammount - expected_balance
            );

            assert_eq!(
                GenericAsset::free_balance(&0, &dest.into()),
                expected_balance
            );
        });
}
