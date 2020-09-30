curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_ask_level",
      "params": ["0xd8deac0ac38775e23437bde7db118437d8e002292327829d772ba4b4c3e0a3","0x6087240931c736c98ccb5748ffe8b0a946fb1bb382db195ad826ec3e060db3d8"]
    }'
#    params: [Blockhash,TradingPair]
#  The params given here must be replaced