# Public Functions
1. register_swap_pair(token0,token1):

    Registers the given tokens for swapping, it will not create the corresponding orderbook based trading pair. It will be called internally by register_trading_pair from Polkadex pallet.

2. swap(inputAmount,minOutputAmount, deadline, path): 

    Swaps the inputAmount based on the given path data
    
3. add_liquidity(inputToken0,inputToken1, minInputToken0, minInputToken1, deadline, swap_pair)
4. remove_liquidity(liquidityTokens,swap_pair, minOutputToken0, minOutputToken1, deadline)
5. swap_for_trade(inputAmount,minOutputAmount, pair): 

    Swaps the inputAmount based on the given pair

# Economics
1. Swap Fees are set at 0.3% per trade. If there are multiple hops in the swap path, each path will charge 0.3%. 
2. The 0.3% is split into 0.2%, 0.005% and 0.095% each is sent to Liquidity Pool, Validator and Foundation respectively.
3. Trades coming from orderbooks are charged 0.2%, where 0.1% goes to Liquidity Pool, 0.005% for Validators and 0.095% for Foundation respectively.  
