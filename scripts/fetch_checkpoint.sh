#!/bin/bash
curl -H "Content-Type: application/json" -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"ob_fetchCheckpoint",
    "params":[]
}' http://localhost:9944
