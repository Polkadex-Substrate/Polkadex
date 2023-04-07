#!/bin/bash

echo "Setting Bootnode Keys"
curl http://localhost:9944 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe1"
curl http://localhost:9944 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran1"
curl http://localhost:9944 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob1"
echo "Setting Validator01 Keys"
curl http://localhost:9946 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe2"
curl http://localhost:9946 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran2"
curl http://localhost:9946 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob2"
echo "Setting Validator02 Keys"
curl http://localhost:9948 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/babe3"
curl http://localhost:9948 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/gran3"
curl http://localhost:9948 -H "Content-Type:application/json;charset=utf-8" -d "@../session-keys/ob3"
