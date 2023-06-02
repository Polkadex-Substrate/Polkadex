for id_n in {4..200}
do
  id=$(subkey inspect "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//babe" | grep Account | awk '0x{print $3}')
  echo "babe: $id"
  cat > ../session-keys/babe$id_n <<-EOM
 {
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
        "babe",
        "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//babe",
        "$id"
    ]
}
EOM
  gran_id=$(subkey inspect --scheme Ed25519 "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//grandpa" | grep Account | awk '0x{print $3}')
  echo "Gran: $gran_id"
  cat > ../session-keys/gran$id_n <<-EOM
{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
        "gran",
        "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//grandpa",
        "$gran_id"
    ]
}
EOM
  ob_id=$(../target/release/bls_gen orderbook $id_n 1)
  echo "Orderbook: $ob_id"
  cat > ../session-keys/ob$id_n <<-EOM
{
    "jsonrpc":"2.0",
    "id":1,
    "method":"author_insertKey",
    "params": [
        "orbk",
        "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//orderbook",
        "$ob_id"
    ]
}
EOM
done
