# Changelog

All notable changes to this project will be documented in this file.

> **Types of changes**
> - `Added` for new features.
> - `Changed` for changes in existing functionality.
> - `Deprecated` for soon-to-be removed features.
> - `Removed` for now removed features.
> - `Fixed` for any bug fixes.
> - `Security` in case of vulnerabilities.

## [Unreleased]

## [5.1.1] - 2023-06-26

### Added

- `thea-message-handler` pallet benchmarks and weights ([#804])
- `update_outgoing_nonce` extrinsic in `thea-message-handler` pallet update outgoing nonce ([#805])
- `update_outgoing_nonce` extrinsic in `thea` pallet to update last processed nonce ([#805])
- `thea-executor` pallet benchmarks and weights ([#802])

### Changed

- `liquidity`, `pallet-ocex-lmp`, `pdex-migration`, `pallet-rewards`, `thea-message-handler`, `thea` pallets weights update ([#806])

### Fixed

- Snapshot generation bug ([#809])

## [5.0.0] - 2023-05-26

### Added

- `thea-client` crate ([#412], [#709], [#728], [#729], [#730], [#732], [#733], [#748], [#750])
- `thea` pallet ([#597], [#595], [#669], [#749], [#750])
- `thea-message-handler` pallet ([#709], [#749], [#750], [#756])
- Github pull request template ([#425])
- `pallet-ocex-lmp` pallet ([#434], [#678], [#741], [#748], [#755], [#759]):
  - Weights ([#486])
  - Tests ([#479], [#748])
  - `remove_proxy_account` extrinsic ([#489])
  - `update_trading_pair` extrinsic ([#522])
  - Token allow-listing ([#527])
  - `set_balances` extrinsic ([#569])
  - `set_snapshot` extrinsic ([#633], [#631])
  - `whitelist_orderbook_operator` extrinsic ([#680])
- Test token provider pallet for testnet ([#424])
- `chainbridge` pallet ([#411])
-  `asset-handler` pallet ([#411], [#753])
- `polkadex-client` crate ([#505])
- Whitelist tokens mechanism for the `chainbridge` ([#523])
- `pallet-asset-handler-runtime-api` crate to perform `Get Balances` RPC calls ([#510], [#528])
- Automated release deployment ([#590])
- `asset-handler` pallet tests ([#602], [#741])
- `PDEX` token handler ([#642])
- `liquidity` pallet ([#624])
- `rewards` pallet ([#622])
- `pallet-amm` pallet ([#634])
- `pallet-amm` pallet benchmarks, weights, tests ([#705])
- `support` pallet ([#634])
- `router` pallet ([#634])
- `thea-executor` pallet ([#706], [#707], [#741], [#757])
- Node compiled binary as an artifact (via CI) to be fetched from other repositories for the testing purpose ([#679])
- Snapshot generation related logic ([#681])
- `Orderbook` client crate ([#693], [#736], [#748], [#755])
- `Orderbook` client tests ([#675], [#721])
- API to check recovery state ([#688])
- Feature to process withdrawals from the snapshot ([#687])
- Signature verifier to process user actions ([#680])
- Calculations for rewards RPC ([#712])
- RPC to fetch `crowdloan` reward information ([#711])
- Runtime build check to the CI ([#715])
- `NoOpConnector` for Dev mode ([#723])
- Documentation for gossiping logic ([#735])
- Changelog ([#747])
- `foreign_chain_url` CLI argument ([#752])

### Changed

- `Udon` testnet genesis config ([#423])
- Added delay for withdrawals above a certain limit in the `asset-handler` pallet ([#490])
- `pallet-ocex-lmp` pallet:
    - In that how withdrawal takes place ([#497])
    - Removed `TradingPairStatus` ([#511])
    - `Balance` handling now uses `Decimal` instead of `Balance` ([#516])
    - `claim_withdraw` extrinsic to make it fee-free ([#556])
- CI/CD to include formatter and `clippy` checks ([#506])
- CI to lint/check formatting of `.toml` files ([#760])
- CI to test weights generation ([#760])
- Fee collection mechanism so that only the `orderbook` council can call it ([#515])
- Error type naming in the `asset-handler` pallet ([#558])
- Bumped `Runtime` and `Client` versions ([#557])
- Dependencies source location changed from branches to tags ([#566])
- Behaviour on block delay ([#571])
- Bumped `polkadex-primitives` as a dependency version in the consuming crates ([#576])
- `spec_name` changed from default ([#585])
- `OrderDetails` struct properties scope to `public` ([#700])
- Bumped `Substrate` version to `v0.9.37` ([#701])
- Bumped `subxt` version to `polkadot-v0.9.37` ([#716])
- Snapshot interval defaults to 20 withdrawals or 5 blocks ([#722])
- `TheaHandler` pallet name in `thea-connector` ([#727])
- Ingress message storage to be map based ([#734])
- Reload last snapshot state in case of Error ([#740])

### Removed

- Old ingress messages ([#723])
- BLS Host functions ([#731])

### Fixed

- Dockerfile to build the `node` by bumping toolchain version ([#409])
- `pallet-ocex-lmp` pallet:
  - To respect `polkadex-primitives` state ([#437])
  - Tests ([#498], [#577], [#694])
  - Minor issues ([#514])
  - `submit_snapshot` extrinsic to not add onchain event if snapshot has empty withdrawals ([#500])
  - `register_trading_pair` extrinsic with adding assertion of minimum volume trading pair ([#524])
  - `update_trading_pair` extrinsic with adding assertion of minimum volume trading pair ([#547])
  - `set_exchange_state` extrinsic ([#550])
  - Withdrawal double spend bug ([#548])
- `mint_asset` extrinsic and amount precision in `asset-handler` pallet ([#473])
- Chain spec ([#504])
- Code style ([#519], [#760])
- Order of assertion in `asset-handler` pallet `mint_asset` extrinsic ([#549])
- Unsafe arithmetic in `pallet-ocex-lmp` and `asset-handler` pallets ([#551])
- Post-release bugs ([#554], [#560])
- Node build in docker ([#568])
- Runtime dependency ([#574])
- Make precision calculation programmable ([#580])
- Benchmarks ([#579])
- Github workflows clean up ([#598])
- Tarpaulin used in the CI ([#627], [#630])
- Removed unused dependencies ([#653], [#699])
- Docker build by pinning toolchain version ([#667])
- Order comparators ([#691])
- `Orderbook` client ([#713], [#710])
- `pallet-rewards-runtime-api` name in runtime `std` feature ([#714])
- Bugs found during `Thea` testing ([#738], [#739])

### Security

- Prevented double spend attack scenario in `collect_fees` extrinsic of `OCEX` pallet ([#552])

[unreleased]: https://github.com/Polkadex-Substrate/Polkadex/compare/v4.0.0...HEAD
[5.0.0]: https://github.com/Polkadex-Substrate/Polkadex/compare/v4.0.0...v5.0.0

[#409]: https://github.com/Polkadex-Substrate/Polkadex/pull/409
[#411]: https://github.com/Polkadex-Substrate/Polkadex/pull/411
[#412]: https://github.com/Polkadex-Substrate/Polkadex/pull/412
[#423]: https://github.com/Polkadex-Substrate/Polkadex/pull/423
[#424]: https://github.com/Polkadex-Substrate/Polkadex/pull/424
[#425]: https://github.com/Polkadex-Substrate/Polkadex/pull/425
[#434]: https://github.com/Polkadex-Substrate/Polkadex/pull/434
[#437]: https://github.com/Polkadex-Substrate/Polkadex/pull/437
[#473]: https://github.com/Polkadex-Substrate/Polkadex/pull/473
[#479]: https://github.com/Polkadex-Substrate/Polkadex/pull/479
[#486]: https://github.com/Polkadex-Substrate/Polkadex/pull/486
[#489]: https://github.com/Polkadex-Substrate/Polkadex/pull/489
[#490]: https://github.com/Polkadex-Substrate/Polkadex/pull/490
[#497]: https://github.com/Polkadex-Substrate/Polkadex/pull/497
[#498]: https://github.com/Polkadex-Substrate/Polkadex/pull/498
[#500]: https://github.com/Polkadex-Substrate/Polkadex/pull/500
[#504]: https://github.com/Polkadex-Substrate/Polkadex/pull/504
[#505]: https://github.com/Polkadex-Substrate/Polkadex/pull/505
[#506]: https://github.com/Polkadex-Substrate/Polkadex/pull/506
[#510]: https://github.com/Polkadex-Substrate/Polkadex/pull/510
[#511]: https://github.com/Polkadex-Substrate/Polkadex/pull/511
[#514]: https://github.com/Polkadex-Substrate/Polkadex/pull/514
[#515]: https://github.com/Polkadex-Substrate/Polkadex/pull/515
[#516]: https://github.com/Polkadex-Substrate/Polkadex/pull/516
[#519]: https://github.com/Polkadex-Substrate/Polkadex/pull/519
[#522]: https://github.com/Polkadex-Substrate/Polkadex/pull/522
[#523]: https://github.com/Polkadex-Substrate/Polkadex/pull/523
[#524]: https://github.com/Polkadex-Substrate/Polkadex/pull/524
[#527]: https://github.com/Polkadex-Substrate/Polkadex/pull/527
[#528]: https://github.com/Polkadex-Substrate/Polkadex/pull/528
[#547]: https://github.com/Polkadex-Substrate/Polkadex/pull/547
[#548]: https://github.com/Polkadex-Substrate/Polkadex/pull/548
[#549]: https://github.com/Polkadex-Substrate/Polkadex/pull/549
[#550]: https://github.com/Polkadex-Substrate/Polkadex/pull/550
[#551]: https://github.com/Polkadex-Substrate/Polkadex/pull/551
[#552]: https://github.com/Polkadex-Substrate/Polkadex/pull/552
[#554]: https://github.com/Polkadex-Substrate/Polkadex/pull/554
[#556]: https://github.com/Polkadex-Substrate/Polkadex/pull/556
[#557]: https://github.com/Polkadex-Substrate/Polkadex/pull/557
[#558]: https://github.com/Polkadex-Substrate/Polkadex/pull/558
[#560]: https://github.com/Polkadex-Substrate/Polkadex/pull/560
[#566]: https://github.com/Polkadex-Substrate/Polkadex/pull/566
[#568]: https://github.com/Polkadex-Substrate/Polkadex/pull/568
[#569]: https://github.com/Polkadex-Substrate/Polkadex/pull/569
[#571]: https://github.com/Polkadex-Substrate/Polkadex/pull/571
[#574]: https://github.com/Polkadex-Substrate/Polkadex/pull/574
[#576]: https://github.com/Polkadex-Substrate/Polkadex/pull/576
[#577]: https://github.com/Polkadex-Substrate/Polkadex/pull/577
[#579]: https://github.com/Polkadex-Substrate/Polkadex/pull/579
[#580]: https://github.com/Polkadex-Substrate/Polkadex/pull/580
[#585]: https://github.com/Polkadex-Substrate/Polkadex/pull/585
[#590]: https://github.com/Polkadex-Substrate/Polkadex/pull/590
[#595]: https://github.com/Polkadex-Substrate/Polkadex/pull/595
[#597]: https://github.com/Polkadex-Substrate/Polkadex/pull/597
[#598]: https://github.com/Polkadex-Substrate/Polkadex/pull/598
[#602]: https://github.com/Polkadex-Substrate/Polkadex/pull/602
[#622]: https://github.com/Polkadex-Substrate/Polkadex/pull/622
[#624]: https://github.com/Polkadex-Substrate/Polkadex/pull/624
[#627]: https://github.com/Polkadex-Substrate/Polkadex/pull/627
[#630]: https://github.com/Polkadex-Substrate/Polkadex/pull/630
[#631]: https://github.com/Polkadex-Substrate/Polkadex/pull/631
[#633]: https://github.com/Polkadex-Substrate/Polkadex/pull/633
[#634]: https://github.com/Polkadex-Substrate/Polkadex/pull/634
[#642]: https://github.com/Polkadex-Substrate/Polkadex/pull/642
[#653]: https://github.com/Polkadex-Substrate/Polkadex/pull/653
[#667]: https://github.com/Polkadex-Substrate/Polkadex/pull/667
[#669]: https://github.com/Polkadex-Substrate/Polkadex/pull/669
[#675]: https://github.com/Polkadex-Substrate/Polkadex/pull/675
[#678]: https://github.com/Polkadex-Substrate/Polkadex/pull/678
[#679]: https://github.com/Polkadex-Substrate/Polkadex/pull/679
[#680]: https://github.com/Polkadex-Substrate/Polkadex/pull/680
[#681]: https://github.com/Polkadex-Substrate/Polkadex/pull/681
[#687]: https://github.com/Polkadex-Substrate/Polkadex/pull/687
[#688]: https://github.com/Polkadex-Substrate/Polkadex/pull/688
[#691]: https://github.com/Polkadex-Substrate/Polkadex/pull/691
[#693]: https://github.com/Polkadex-Substrate/Polkadex/pull/693
[#694]: https://github.com/Polkadex-Substrate/Polkadex/pull/694
[#699]: https://github.com/Polkadex-Substrate/Polkadex/pull/699
[#700]: https://github.com/Polkadex-Substrate/Polkadex/pull/700
[#701]: https://github.com/Polkadex-Substrate/Polkadex/pull/701
[#705]: https://github.com/Polkadex-Substrate/Polkadex/pull/705
[#706]: https://github.com/Polkadex-Substrate/Polkadex/pull/706
[#707]: https://github.com/Polkadex-Substrate/Polkadex/pull/707
[#709]: https://github.com/Polkadex-Substrate/Polkadex/pull/709
[#710]: https://github.com/Polkadex-Substrate/Polkadex/pull/710
[#711]: https://github.com/Polkadex-Substrate/Polkadex/pull/711
[#712]: https://github.com/Polkadex-Substrate/Polkadex/pull/712
[#713]: https://github.com/Polkadex-Substrate/Polkadex/pull/713
[#714]: https://github.com/Polkadex-Substrate/Polkadex/pull/714
[#715]: https://github.com/Polkadex-Substrate/Polkadex/pull/715
[#716]: https://github.com/Polkadex-Substrate/Polkadex/pull/716
[#721]: https://github.com/Polkadex-Substrate/Polkadex/pull/721
[#722]: https://github.com/Polkadex-Substrate/Polkadex/pull/722
[#723]: https://github.com/Polkadex-Substrate/Polkadex/pull/723
[#727]: https://github.com/Polkadex-Substrate/Polkadex/pull/727
[#728]: https://github.com/Polkadex-Substrate/Polkadex/pull/728
[#729]: https://github.com/Polkadex-Substrate/Polkadex/pull/729
[#730]: https://github.com/Polkadex-Substrate/Polkadex/pull/730
[#731]: https://github.com/Polkadex-Substrate/Polkadex/pull/731
[#732]: https://github.com/Polkadex-Substrate/Polkadex/pull/732
[#733]: https://github.com/Polkadex-Substrate/Polkadex/pull/733
[#734]: https://github.com/Polkadex-Substrate/Polkadex/pull/734
[#735]: https://github.com/Polkadex-Substrate/Polkadex/pull/735
[#736]: https://github.com/Polkadex-Substrate/Polkadex/pull/736
[#738]: https://github.com/Polkadex-Substrate/Polkadex/pull/738
[#739]: https://github.com/Polkadex-Substrate/Polkadex/pull/739
[#740]: https://github.com/Polkadex-Substrate/Polkadex/pull/740
[#741]: https://github.com/Polkadex-Substrate/Polkadex/pull/741
[#747]: https://github.com/Polkadex-Substrate/Polkadex/pull/747
[#748]: https://github.com/Polkadex-Substrate/Polkadex/pull/748
[#749]: https://github.com/Polkadex-Substrate/Polkadex/pull/749
[#750]: https://github.com/Polkadex-Substrate/Polkadex/pull/750
[#752]: https://github.com/Polkadex-Substrate/Polkadex/pull/752
[#753]: https://github.com/Polkadex-Substrate/Polkadex/pull/753
[#755]: https://github.com/Polkadex-Substrate/Polkadex/pull/755
[#756]: https://github.com/Polkadex-Substrate/Polkadex/pull/756
[#757]: https://github.com/Polkadex-Substrate/Polkadex/pull/757
[#759]: https://github.com/Polkadex-Substrate/Polkadex/pull/759
[#760]: https://github.com/Polkadex-Substrate/Polkadex/pull/760
[#804]: https://github.com/Polkadex-Substrate/Polkadex/pull/804
[#805]: https://github.com/Polkadex-Substrate/Polkadex/pull/805
[#806]: https://github.com/Polkadex-Substrate/Polkadex/pull/806
[#809]: https://github.com/Polkadex-Substrate/Polkadex/pull/809
[#802]: https://github.com/Polkadex-Substrate/Polkadex/pull/802
