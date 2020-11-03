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
use crate::mock::{Assets, ExtBuilder, Origin, System, TestEvent, TokenDealer};
use frame_support::assert_ok;

// TODO test not enough balance fails, may need to add DispatchResult for functions
#[test]
fn transfer_to_relay_chain_settles_accounts_on_parachain_with_event() {
    // 1 DOT = 1000 spending asset, 1 spending asset = 1 generic asset
    // let assets_relay_rates = (1000, 1);
    let initial_para_amount = 10000;
    let transfer_amount = 1000;
    let from = [0u8; 32];
    let to = [1u8; 32];
    let relay_account: [u8; 32] = RelayId::default().into_account();
    let expected_balance = initial_para_amount - transfer_amount;
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensToRelayChain(
        relay_account.into(),
        to.into(),
        transfer_amount,
    ));

    ExtBuilder::default()
        // assetid, account, balance
        .free_balance(vec![(from.into(), initial_para_amount)])
        // spending to relay, generic to spending
        .build()
        .execute_with(|| {
            assert_ok!(Assets::issue(
                Origin::signed(from.into()),
                initial_para_amount
            ));
            assert_ok!(TokenDealer::transfer_tokens_to_relay_chain(
                Origin::signed(from.into()),
                to.into(),
                transfer_amount,
                Some(0)
            ));
            assert_eq!(Assets::balance(0, from.into()), expected_balance);
            assert_eq!(Assets::balance(0, relay_account.into()), transfer_amount);
            assert!(System::events()
                .iter()
                .any(|record| record.event == expected_event));
        });
}

// #[test]
// fn downward_message_settles_accounts_on_parachain_with_event() {
//     // 1 DOT = 1000 spending asset, 1 spending asset = 1 generic asset
//     let assets_relay_rates = (1000, 1);
//     let initial_relay_account_ammount = 10000;
//     let transfer_relay_amount = 9;
//     let expected_balance = transfer_relay_amount * assets_relay_rates.0;
//
//     let dest = [0u8; 32];
//     let remark = [0u8; 32];
//     let relay_account: [u8; 32] = RelayId::default().into_account();
//     let downward_message =
//         DownwardMessage::TransferInto(dest.into(), transfer_relay_amount, remark);
//     let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensFromRelayChain(
//         dest.into(),
//         transfer_relay_amount * assets_relay_rates.0,
//         Ok(()),
//     ));
//     ExtBuilder::default()
//         // assetid, account, balance
//         .free_balance((0, relay_account.into(), initial_relay_account_ammount))
//         .assets_relay_rates((1000, 1))
//         // spending to relay, generic to spending
//         .build()
//         .execute_with(|| {
//             TokenDealer::handle_downward_message(&downward_message);
//             assert_eq!(
//                 GenericAsset::free_balance(&0, &relay_account.into()),
//                 initial_relay_account_ammount - expected_balance
//             );
//             assert_eq!(
//                 GenericAsset::free_balance(&0, &dest.into()),
//                 expected_balance
//             );
//             assert!(System::events()
//                 .iter()
//                 .any(|record| record.event == expected_event));
//         });
// }
//
// #[test]
// fn transfer_tokens_to_para_settles_accounts_on_parachain_with_event() {
//     let from = [0u8; 32];
//     let initial_account_ammount = 10000;
//     let transfer_amount = 9000;
//     let asset_id = 0;
//
//     let para_id: ParaId = 200.into();
//
//     let dest = [0u8; 32];
//
//     let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensToParachain(
//         para_id,
//         para_id.into_account(),
//         dest.into(),
//         transfer_amount,
//         asset_id,
//     ));
//     ExtBuilder::default()
//         // assetid, account, balance
//         .free_balance((0, from.into(), initial_account_ammount))
//         // spending to relay, generic to spending
//         .build()
//         .execute_with(|| {
//             assert_ok!(TokenDealer::transfer_assets_to_parachain_chain(
//                 Origin::signed(from.into()),
//                 para_id.into(),
//                 dest.into(),
//                 transfer_amount,
//                 asset_id,
//             ));
//             assert_eq!(
//                 GenericAsset::free_balance(&0, &para_id.into_account()),
//                 transfer_amount
//             );
//             assert_eq!(
//                 GenericAsset::free_balance(&0, &from.into()),
//                 initial_account_ammount - transfer_amount
//             );
//             assert!(System::events()
//                 .iter()
//                 .any(|record| record.event == expected_event));
//         });
// }
//
// #[test]
// fn handle_xcmp_transfer_token_message_settles_accounts_on_parachain_with_event() {
//     let dest = [0u8; 32];
//     let initial_account_ammount = 10000;
//     let transfer_amount = 9000;
//     let asset_id = 0;
//     let msg = XCMPMessage::TransferToken(dest.into(), transfer_amount);
//     let para_id: ParaId = 200.into();
//     let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensViaXCMP(
//         para_id,
//         dest.into(),
//         transfer_amount,
//         asset_id,
//         Ok(()),
//     ));
//
//     ExtBuilder::default()
//         // assetid, account, balance
//         .free_balance((0, para_id.into_account(), initial_account_ammount))
//         // spending to relay, generic to spending
//         .build()
//         .execute_with(|| {
//             TokenDealer::handle_xcmp_message(para_id, &msg);
//             assert_eq!(
//                 GenericAsset::free_balance(
//                     &GenericAsset::spending_asset_id(),
//                     &para_id.into_account()
//                 ),
//                 initial_account_ammount - transfer_amount
//             );
//             assert_eq!(
//                 GenericAsset::free_balance(&GenericAsset::spending_asset_id(), &dest.into()),
//                 transfer_amount
//             );
//             assert!(System::events()
//                 .iter()
//                 .any(|record| record.event == expected_event));
//         });
// }
//
// #[test]
// fn handle_xcmp_transfer_assets_message_settles_accounts_on_parachain_with_event() {
//     let dest = [0u8; 32];
//     let initial_account_ammount = 10000;
//     let transfer_amount = 9000;
//     let asset_id = 9;
//     let msg = XCMPMessage::TransferAsset(dest.into(), transfer_amount, asset_id);
//     let para_id: ParaId = 200.into();
//     let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensViaXCMP(
//         para_id,
//         dest.into(),
//         transfer_amount,
//         asset_id,
//         Ok(()),
//     ));
//
//     ExtBuilder::default()
//         // assetid, account, balance
//         .free_balance((asset_id, para_id.into_account(), initial_account_ammount))
//         // spending to relay, generic to spending
//         .build()
//         .execute_with(|| {
//             TokenDealer::handle_xcmp_message(para_id, &msg);
//             assert_eq!(
//                 GenericAsset::free_balance(&asset_id, &para_id.into_account()),
//                 initial_account_ammount - transfer_amount
//             );
//             assert_eq!(
//                 GenericAsset::free_balance(&asset_id, &dest.into()),
//                 transfer_amount
//             );
//             assert!(System::events()
//                 .iter()
//                 .any(|record| record.event == expected_event));
//         });
// }
