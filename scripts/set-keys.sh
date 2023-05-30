#!/bin/bash

for id in {1..200}
do
  port=$((9943 + $id))
  echo "Setting $id Keys with RPC: $port"
  curl http://localhost:$port -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe$id"
  curl http://localhost:$port -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran$id"
  curl http://localhost:$port -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob$id"
done
