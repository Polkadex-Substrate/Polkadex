#!/bin/bash
curl -H "Content-Type: application/json" -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"ob_getBalance",
    "params":["esqQ4BtvmTW9J5pXaqkomFmPh9qQgiogYaokE8uWUJXA3ThJq",{"asset":"226557799181424065994173367616174607641"}]
}' http://localhost:9944
