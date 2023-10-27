#!/bin/bash
curl -H "Content-Type: application/json" -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"ob_getBalance",
    "params":["esqcMH7tgSpFQ8vivW42yZezgEzxD1ye9YXaW7MLDtQb1KNs4",{"asset":"3496813586714279103986568049643838918"}]
}' http://localhost:9944
