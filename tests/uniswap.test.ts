import { expect } from 'chai';
import { network, patract } from 'redspot';

const { getContractFactory, getRandomSigner } = patract;
const { api, getSigners } = network;

describe('UniswapV2', () => {
  after(() => {
    return api.disconnect();
  });

  async function setup() {
    await api.isReady;
    const Alice = (await getSigners())[0];
    const Bob = (await getSigners())[1];
    const Charlie = (await getSigners())[2];

    const contractFactory = await getContractFactory('uniswap_v2', Alice);
    const contract = await contractFactory.deploy('new');

    await api.setSigner(Alice);

    const tokenA = '0x000000000000000000000000000000000000000a';
    const tokenB = '0x000000000000000000000000000000000000000b';
    const tokenC = '0x000000000000000000000000000000000000000c';

    const asyncSudoCall = (call) => {
      return new Promise(async (resolve, reject) => {
        await api.tx.sudo
          .sudo(call)
          .signAndSend(Alice.address, ({ status }) => {
            if (status.isFinalized) {
              resolve(status);
            } else if (
              status.isInvalid ||
              status.isFinalityTimeout ||
              status.isUsurped
            ) {
              reject(status);
            }
          });
      });
    };

    const addAccountBalance = async (address, tokenAddr, amount) => {
      await asyncSudoCall(
        api.tx.currencies.updateBalance(
          address,
          { chainsafe: tokenAddr },
          amount
        )
      );
    };

    const getAccountBalance = async (address, tokenAddr) => {
      const balance = await api.query.tokens.accounts(address, {
        chainsafe: tokenAddr
      });
      return JSON.parse(balance.toString()).free;
    };

    return {
      contractFactory,
      contract,
      Alice,
      Bob,
      Charlie,
      tokenA,
      tokenB,
      tokenC,
      addAccountBalance,
      getAccountBalance
    };
  }

  it('Add liquidity works', async () => {
    const {
      contract,
      Alice,
      Bob,
      tokenA,
      tokenB,
      addAccountBalance,
      getAccountBalance
    } = await setup();

    const AMOUNT_A = 100_000,
      AMOUNT_B = 20_000,
      AMOUNT_C = 20_000,
      AMOUNT_D = 40_000;

    const orgBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const orgBobBalanceB = await getAccountBalance(Bob.address, tokenB);

    if (orgBobBalanceA < AMOUNT_A) {
      await addAccountBalance(Bob.address, tokenA, AMOUNT_A);
    }

    if (orgBobBalanceB < AMOUNT_B) {
      await addAccountBalance(Bob.address, tokenB, AMOUNT_B);
    }

    const orgAliceBalanceA = await getAccountBalance(Alice.address, tokenA);
    const orgAliceBalanceB = await getAccountBalance(Alice.address, tokenB);

    if (orgAliceBalanceA < AMOUNT_C) {
      await addAccountBalance(Alice.address, tokenA, AMOUNT_C);
    }

    if (orgAliceBalanceB < AMOUNT_D) {
      await addAccountBalance(Alice.address, tokenB, AMOUNT_D);
    }

    // Initial liquidity should be [0, 0]
    let liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output).to.eq([0, 0]);

    // addLiquidity tx should emit LiquidityAdded event with provided args
    await expect(
      contract
        .connect(Bob)
        .tx.addLiquidity(tokenA, tokenB, AMOUNT_A, AMOUNT_B, 0, true)
    )
      .to.emit(contract, 'LiquidityAdded')
      .withArgs(Bob.address, tokenA, AMOUNT_A, tokenB, AMOUNT_B, 200_000);

    // [tokenA, tokenB] & [tokenB, tokenA] liquidity should be set properly
    liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output).to.eq([AMOUNT_A, AMOUNT_B]);

    liquidity = await contract.query.getLiquidity(tokenB, tokenA);
    expect(liquidity.output).to.eq([AMOUNT_B, AMOUNT_A]);

    // totalIssuance should be set properly
    let totalIssuance = await contract.query.getTotalIssuance(tokenA, tokenB);
    expect(totalIssuance.output).to.eq(200_000);

    // dexIncentive should be set to only Bob, not for Alice
    let dexIncentiveAlice = await contract.query.getDexIncentive(
      tokenA,
      tokenB,
      Alice.address
    );
    expect(dexIncentiveAlice.output).to.eq(0);

    let dexIncentiveBob = await contract.query.getDexIncentive(
      tokenA,
      tokenB,
      Bob.address
    );
    expect(dexIncentiveBob.output).to.eq(200_000);

    // should transfer actual balance from account Bob
    const newBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const newBobBalanceB = await getAccountBalance(Bob.address, tokenB);
    expect(newBobBalanceA).to.eq(orgBobBalanceA - AMOUNT_A);
    expect(newBobBalanceB).to.eq(orgBobBalanceB - AMOUNT_B);

    // Add liquidity from Alice again
    await expect(
      contract
        .connect(Alice)
        .tx.addLiquidity(tokenA, tokenB, AMOUNT_C, AMOUNT_D, 0, true)
    )
      .to.emit(contract, 'LiquidityAdded')
      .withArgs(Alice.address, tokenA, 20_000, tokenB, 3_999, 39_999);

    // [tokenA, tokenB] liquidity should be set properly
    liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output).to.eq([AMOUNT_A + 20_000, AMOUNT_B + 3_999]);

    // totalIssuance should be set properly
    totalIssuance = await contract.query.getTotalIssuance(tokenA, tokenB);
    expect(totalIssuance.output).to.eq(239_999);

    // dexIncentive should be set properly
    dexIncentiveAlice = await contract.query.getDexIncentive(
      tokenA,
      tokenB,
      Alice.address
    );
    expect(dexIncentiveAlice.output).to.eq(39_999);
  });

  it('Remove liquidity works', async () => {
    const {
      contract,
      Alice,
      Bob,
      tokenA,
      tokenB,
      addAccountBalance,
      getAccountBalance
    } = await setup();
    const AMOUNT_A = 100;
    const AMOUNT_B = 20;
    const TOTAL_SHARE = 200;
    const REMOVE_SHARE = 50;
    const DECREMENT_A = 25;
    const DECREMENT_B = 5;

    const orgBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const orgBobBalanceB = await getAccountBalance(Bob.address, tokenB);

    if (orgBobBalanceA < AMOUNT_A) {
      await addAccountBalance(Bob.address, tokenA, AMOUNT_A);
    }

    if (orgBobBalanceB < AMOUNT_B) {
      await addAccountBalance(Bob.address, tokenB, AMOUNT_B);
    }

    await contract
      .connect(Bob)
      .tx.addLiquidity(tokenA, tokenB, AMOUNT_A, AMOUNT_B, 0, true);

    await expect(
      contract
        .connect(Bob)
        .tx.removeLiquidity(tokenA, tokenB, REMOVE_SHARE, 0, 0, true)
    )
      .to.emit(contract, 'LiquidityRemoved')
      .withArgs(
        Bob.address,
        tokenA,
        DECREMENT_A,
        tokenB,
        DECREMENT_B,
        REMOVE_SHARE
      );

    // [tokenA, tokenB] & [tokenB, tokenA] liquidity should be updated properly
    let liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output).to.eq([
      AMOUNT_A - DECREMENT_A,
      AMOUNT_B - DECREMENT_B
    ]);

    // totalIssuance should be updated properly
    let totalIssuance = await contract.query.getTotalIssuance(tokenA, tokenB);
    expect(totalIssuance.output).to.eq(TOTAL_SHARE - REMOVE_SHARE);

    // dexIncentive should be updated properly
    let dexIncentiveBob = await contract.query.getDexIncentive(
      tokenA,
      tokenB,
      Bob.address
    );
    expect(dexIncentiveBob.output).to.eq(TOTAL_SHARE - REMOVE_SHARE);

    // should withdraw actual balance from liquidity pool
    const newBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const newBobBalanceB = await getAccountBalance(Bob.address, tokenB);
    expect(newBobBalanceA).to.eq(orgBobBalanceA - AMOUNT_A + DECREMENT_A);
    expect(newBobBalanceB).to.eq(orgBobBalanceB - AMOUNT_B + DECREMENT_B);
  });

  it.only('Single Path Swap works', async () => {
    const {
      contract,
      Alice,
      Bob,
      tokenA,
      tokenB,
      addAccountBalance,
      getAccountBalance
    } = await setup();

    const AMOUNT_A = 100_000,
      AMOUNT_B = 20_000,
      AMOUNT_C = 20_000,
      AMOUNT_D = 40_000;

    const orgBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const orgBobBalanceB = await getAccountBalance(Bob.address, tokenB);

    if (orgBobBalanceA < AMOUNT_A) {
      await addAccountBalance(Bob.address, tokenA, AMOUNT_A);
    }

    if (orgBobBalanceB < AMOUNT_B) {
      await addAccountBalance(Bob.address, tokenB, AMOUNT_B);
    }

    const orgAliceBalanceA = await getAccountBalance(Alice.address, tokenA);
    const orgAliceBalanceB = await getAccountBalance(Alice.address, tokenB);

    if (orgAliceBalanceA < AMOUNT_C) {
      await addAccountBalance(Alice.address, tokenA, AMOUNT_C);
    }

    if (orgAliceBalanceB < AMOUNT_D) {
      await addAccountBalance(Alice.address, tokenB, AMOUNT_D);
    }

    await contract
      .connect(Bob)
      .tx.addLiquidity(tokenA, tokenB, AMOUNT_A, AMOUNT_B, 0, true);

    // test targetAmount from supply
    const testTargetAmount = await contract
      .connect(Alice)
      .query.getSwapTargetAmount([tokenA, tokenB], 20_000);

    expect(testTargetAmount.output).to.eq(3_324);

    // test supplyAmount from target
    const testSupplyAmount = await contract
      .connect(Alice)
      .query.getSwapSupplyAmount([tokenA, tokenB], 3_324);

    expect(testSupplyAmount.output).to.eq(19_993);

    await expect(
      contract
        .connect(Alice)
        .tx.swapWithExactSupply([tokenA, tokenB], 20_000, 0)
    )
      .to.emit(contract, 'Swap')
      .withArgs(Alice.address, [tokenA, tokenB], 20_000, 3_324);

    // should transfer actual balance
    const newAliceBalanceA = await getAccountBalance(Alice.address, tokenA);
    const newAliceBalanceB = await getAccountBalance(Alice.address, tokenB);
    expect(newAliceBalanceA).to.eq(orgAliceBalanceA - 20_000);
    expect(newAliceBalanceB).to.eq(orgAliceBalanceB + 3_324);

    await expect(
      contract
        .connect(Alice)
        .tx.swapWithExactTarget([tokenB, tokenA], 10_000, 2_000)
    )
      .to.emit(contract, 'Swap')
      .withArgs(Alice.address, [tokenB, tokenA], 1_521, 10_000);

    // should transfer actual balance
    const newAliceBalanceA_2 = await getAccountBalance(Alice.address, tokenA);
    const newAliceBalanceB_2 = await getAccountBalance(Alice.address, tokenB);
    expect(newAliceBalanceA_2).to.eq(newAliceBalanceA + 10_000);
    expect(newAliceBalanceB_2).to.eq(newAliceBalanceB - 1_521);
  });
});
