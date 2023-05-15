for id_n in {4..200}
do
  id=$(subkey inspect "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//babe" | grep Account | awk '0x{print $3}')
  echo "babe: $id"
  cat > ../session-keys/babe$id_n <<-EOM
 {
    "jsonrpc":"2.0",
    "id":$id_n,
    "method":"author_insertKey",
    "params": [
        "babe",
        "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//babe",
        "$id"
    ]
}
EOM
  gran_id=$(subkey inspect "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//grandpa" | grep Account | awk '0x{print $3}')
  echo "Gran: $gran_id"
  cat > ../session-keys/gran$id_n <<-EOM
{
    "jsonrpc":"2.0",
    "id":$id_n,
    "method":"author_insertKey",
    "params": [
        "gran",
        "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//grandpa",
        "$gran_id"
    ]
}
EOM
  ob_id=$(subkey inspect "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//grandpa" | grep Account | awk '0x{print $3}')
  echo "Orderbook: $ob_id"
  cat > ../session-keys/ob$id_n <<-EOM
{
    "jsonrpc":"2.0",
    "id":$id_n,
    "method":"author_insertKey",
    "params": [
        "orbk",
        "owner word vocal dose decline sunset battle example forget excite gentle waste//$id_n//orderbook",
        "0xaee672d32bf85ef55c5fecedc0cc4c17ab828e4e30cada4a565fc0136c4adc1cdad56efb96d52ac19d84b43f289ad456127ea3d09af51cef4ff0776375c1ea0d2b6c42dd095119ff371e8d3d56f44eca23811d19298755dba7627fd61a3f0c9e"
    ]
}
done
