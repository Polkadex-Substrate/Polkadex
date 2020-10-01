curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_ask_level",
      "params": ["0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9","0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9"]
    }'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_bid_level",
      "params": ["0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9","0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9"]
    }'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_orderbook",
      "params": ["0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9","0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9"]
    }'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_market_info",
      "params": ["0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9","0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9",354]
    }'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
     "jsonrpc":"2.0",
      "id":1,
      "method":"get_all_orderbook",
      "params": ["0xf28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9"]
    }'