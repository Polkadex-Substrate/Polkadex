import chai from 'chai';
import { network, patract } from 'redspot';
import BN from 'bn.js';
const expect = chai.expect;
chai.use(require('chai-as-promised'));

const { getContractFactory } = patract;
const { api, getSigners } = network;

function expandTo18Decimals(n): BN {
  return new BN(n).mul(new BN(10).pow(new BN(18)));
}

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
        await api.tx.sudo.sudo(call).signAndSend(Alice.address, ({ status }) => {
          if (status.isFinalized) {
            resolve(status);
          } else if (status.isInvalid || status.isFinalityTimeout || status.isUsurped) {
            reject(status);
          }
        });
      });
    };

    const addAccountBalance = async (address, tokenAddr, amount) => {
      await asyncSudoCall(api.tx.currencies.updateBalance(address, { chainsafe: tokenAddr }, amount));
    };

    const getAccountBalance = async (address, tokenAddr) => {
      const balance = await api.query.tokens.accounts(address, {
        chainsafe: tokenAddr
      });
      return new BN(balance.free.toString());
    };

    const prepareBalance = async (address, token0Amount: BN, token1Amount: BN) => {
      const balance0 = await getAccountBalance(address, tokenA);
      const balance1 = await getAccountBalance(address, tokenB);

      if (balance0.cmp(token0Amount) < 0) {
        await addAccountBalance(address, tokenA, token0Amount);
      }

      if (balance1.cmp(token1Amount) < 0) {
        await addAccountBalance(address, tokenB, token1Amount);
      }
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
      getAccountBalance,
      prepareBalance
    };
  }

  it('Add liquidity works', async () => {
    const { contract, Alice, Bob, tokenA, tokenB, getAccountBalance, prepareBalance } = await setup();

    const AMOUNT_A = new BN(100000),
      AMOUNT_B = new BN(20000),
      AMOUNT_C = new BN(20000),
      AMOUNT_D = new BN(40000);

    await prepareBalance(Bob.address, AMOUNT_A, AMOUNT_B);

    const orgBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const orgBobBalanceB = await getAccountBalance(Bob.address, tokenB);

    await prepareBalance(Alice.address, AMOUNT_A, AMOUNT_B);

    await getAccountBalance(Alice.address, tokenA);
    await getAccountBalance(Alice.address, tokenB);

    // Initial liquidity should be [0, 0]
    let liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output).to.eq([0, 0]);

    // addLiquidity tx should emit LiquidityAdded event with provided args
    await expect(contract.connect(Bob).tx.addLiquidity(tokenA, tokenB, AMOUNT_A, AMOUNT_B, 0, true))
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
    let dexIncentiveAlice = await contract.query.getDexIncentive(tokenA, tokenB, Alice.address);
    expect(dexIncentiveAlice.output).to.eq(0);

    let dexIncentiveBob = await contract.query.getDexIncentive(tokenA, tokenB, Bob.address);
    expect(dexIncentiveBob.output).to.eq(200_000);

    // should transfer actual balance from account Bob
    const newBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const newBobBalanceB = await getAccountBalance(Bob.address, tokenB);
    expect(newBobBalanceA).to.eq(orgBobBalanceA.sub(AMOUNT_A));
    expect(newBobBalanceB).to.eq(orgBobBalanceB.sub(AMOUNT_B));

    // Add liquidity from Alice again
    await expect(contract.connect(Alice).tx.addLiquidity(tokenA, tokenB, AMOUNT_C, AMOUNT_D, 0, true))
      .to.emit(contract, 'LiquidityAdded')
      .withArgs(Alice.address, tokenA, 20_000, tokenB, 3_999, 39_999);

    // [tokenA, tokenB] liquidity should be set properly
    liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output).to.eq([AMOUNT_A.add(new BN(20_000)), AMOUNT_B.add(new BN(3_999))]);

    // totalIssuance should be set properly
    totalIssuance = await contract.query.getTotalIssuance(tokenA, tokenB);
    expect(totalIssuance.output).to.eq(239_999);

    // dexIncentive should be set properly
    dexIncentiveAlice = await contract.query.getDexIncentive(tokenA, tokenB, Alice.address);
    expect(dexIncentiveAlice.output).to.eq(39_999);
  });

  it('Add liquidity works with BigNumber', async () => {
    const swapTestCases = [
      [1, 10, 5, '453305446940074565'],
      [1, 5, 10, '1662497915624478906'],

      [2, 5, 10, '2851015155847869602'],
      [2, 10, 5, '831248957812239453'],

      [1, 10, 10, '906610893880149131'],
      [1, 100, 100, '987158034397061298'],
      [1, 1000, 1000, '996006981039903216']
    ].map((a) => a.map((n, i) => (i > 2 ? new BN(n) : expandTo18Decimals(n))));

    for (let swapTestCase of swapTestCases) {
      const { contract, Bob, tokenA, tokenB, prepareBalance } = await setup();
      const [swapAmount, token0Amount, token1Amount, expectedOutputAmount] = swapTestCase;
      await prepareBalance(Bob.address, token0Amount, token1Amount);

      await contract.connect(Bob).tx.addLiquidity(tokenA, tokenB, token0Amount, token1Amount, 0, true);

      const testTargetAmount = await contract.connect(Bob).query.getSwapTargetAmount([tokenA, tokenB], swapAmount);
      console.log(testTargetAmount.output?.toString());
      expect(testTargetAmount.output).to.eq(expectedOutputAmount.toString());
    }
  }).timeout(200000);

  it('Remove liquidity works', async () => {
    const { contract, Bob, tokenA, tokenB, getAccountBalance, prepareBalance } = await setup();
    const AMOUNT_A = expandTo18Decimals(100);
    const AMOUNT_B = expandTo18Decimals(20);
    const TOTAL_SHARE = expandTo18Decimals(200);
    const REMOVE_SHARE = expandTo18Decimals(50);
    const DECREMENT_A = expandTo18Decimals(25);
    const DECREMENT_B = expandTo18Decimals(5);

    await prepareBalance(Bob.address, AMOUNT_A, AMOUNT_B);

    const orgBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const orgBobBalanceB = await getAccountBalance(Bob.address, tokenB);

    await contract.connect(Bob).tx.addLiquidity(tokenA, tokenB, AMOUNT_A, AMOUNT_B, 0, true);

    await expect(contract.connect(Bob).tx.removeLiquidity(tokenA, tokenB, REMOVE_SHARE, 0, 0, true))
      .to.emit(contract, 'LiquidityRemoved')
      .withArgs(Bob.address, tokenA, DECREMENT_A, tokenB, DECREMENT_B, REMOVE_SHARE);

    // [tokenA, tokenB] & [tokenB, tokenA] liquidity should be updated properly
    let liquidity = await contract.query.getLiquidity(tokenA, tokenB);
    expect(liquidity.output[0].toString()).to.eq(AMOUNT_A.sub(DECREMENT_A).toString());
    expect(liquidity.output[1].toString()).to.eq(AMOUNT_B.sub(DECREMENT_B).toString());

    // totalIssuance should be updated properly
    let totalIssuance = await contract.query.getTotalIssuance(tokenA, tokenB);
    expect(totalIssuance.output).to.eq(TOTAL_SHARE.sub(REMOVE_SHARE));

    // dexIncentive should be updated properly
    let dexIncentiveBob = await contract.query.getDexIncentive(tokenA, tokenB, Bob.address);
    expect(dexIncentiveBob.output).to.eq(TOTAL_SHARE.sub(REMOVE_SHARE));

    // should withdraw actual balance from liquidity pool
    const newBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const newBobBalanceB = await getAccountBalance(Bob.address, tokenB);
    expect(newBobBalanceA).to.eq(orgBobBalanceA.sub(AMOUNT_A).add(DECREMENT_A));
    expect(newBobBalanceB).to.eq(orgBobBalanceB.sub(AMOUNT_B).add(DECREMENT_B));
  });

  it('Single Path Swap works', async () => {
    const { contract, Alice, Bob, tokenA, tokenB, getAccountBalance, prepareBalance } = await setup();

    const AMOUNT_A = expandTo18Decimals(100),
      AMOUNT_B = expandTo18Decimals(20),
      AMOUNT_C = expandTo18Decimals(20),
      AMOUNT_D = expandTo18Decimals(40);

    await prepareBalance(Bob.address, AMOUNT_A, AMOUNT_B);

    await getAccountBalance(Bob.address, tokenA);
    await getAccountBalance(Bob.address, tokenB);

    await prepareBalance(Alice.address, AMOUNT_C, AMOUNT_D);

    const orgAliceBalanceA = await getAccountBalance(Alice.address, tokenA);
    const orgAliceBalanceB = await getAccountBalance(Alice.address, tokenB);
    await contract.connect(Bob).tx.addLiquidity(tokenA, tokenB, AMOUNT_A, AMOUNT_B, 0, true);
    // test targetAmount from supply
    const testTargetAmount = await contract
      .connect(Alice)
      .query.getSwapTargetAmount([tokenA, tokenB], expandTo18Decimals(20));

    expect(testTargetAmount.output?.toString()).to.eq('3324995831248957812');

    // test supplyAmount from target
    const testSupplyAmount = await contract
      .connect(Alice)
      .query.getSwapSupplyAmount([tokenA, tokenB], new BN('3324995831248957812'));
    expect(testSupplyAmount.output?.toString()).to.eq('19999999999999999999');

    await expect(contract.connect(Alice).tx.swapWithExactSupply([tokenA, tokenB], expandTo18Decimals(20), 0))
      .to.emit(contract, 'Swap')
      .withArgs(Alice.address, [tokenA, tokenB], expandTo18Decimals(20), new BN('3324995831248957812'));

    // should transfer actual balance
    const newAliceBalanceA = await getAccountBalance(Alice.address, tokenA);
    const newAliceBalanceB = await getAccountBalance(Alice.address, tokenB);
    expect(newAliceBalanceA).to.eq(orgAliceBalanceA.sub(expandTo18Decimals(20)));
    expect(newAliceBalanceB).to.eq(orgAliceBalanceB.add(new BN('3324995831248957812')));

    await expect(
      contract.connect(Alice).tx.swapWithExactTarget([tokenB, tokenA], expandTo18Decimals(10), expandTo18Decimals(2))
    )
      .to.emit(contract, 'Swap')
      .withArgs(Alice.address, [tokenB, tokenA], new BN('1520470882534060563'), expandTo18Decimals(10));

    // should transfer actual balance
    const newAliceBalanceA_2 = await getAccountBalance(Alice.address, tokenA);
    const newAliceBalanceB_2 = await getAccountBalance(Alice.address, tokenB);
    expect(newAliceBalanceA_2).to.eq(newAliceBalanceA.add(expandTo18Decimals(10)));
    expect(newAliceBalanceB_2).to.eq(newAliceBalanceB.sub(new BN('1520470882534060563')));
  });

  it('Transaction should revert all ran actions when it is in panic', async () => {
    const { contract, Bob, tokenA, tokenB, getAccountBalance, prepareBalance } = await setup();

    await prepareBalance(Bob.address, new BN(100), new BN(100));

    const orgBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const orgBobBalanceB = await getAccountBalance(Bob.address, tokenB);

    await expect(
      contract.connect(Bob).tx.addLiquidity(tokenA, tokenB, orgBobBalanceA, orgBobBalanceB.add(new BN(1)), 0, true)
    ).to.be.rejected;

    const newBobBalanceA = await getAccountBalance(Bob.address, tokenA);
    const newBobBalanceB = await getAccountBalance(Bob.address, tokenB);
    expect(newBobBalanceA).to.eq(orgBobBalanceA);
    expect(newBobBalanceB).to.eq(orgBobBalanceB);
  });
});