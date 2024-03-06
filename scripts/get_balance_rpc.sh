#!/bin/bash
curl -H "Content-Type: application/json" -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"ob_getBalance",
    "params":["esntiQCvLG55kFN3cqJjQoA1VeBrx9oiorDobuejDta7xcokf",{"asset":"3496813586714279103986568049643838918"}]
}' http://localhost:9944
