#!/bin/bash
curl -H "Content-Type: application/json" -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"ob_getBalance",
    "params":["espg8YLUhUDVXcrfNMyHqzjBBrrq3HBKj3FBQNq6Hte3p28Eh",{"asset":"95930534000017180603917534864279132680"}]
}' http://localhost:9944
