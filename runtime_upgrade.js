const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const types = require('./typedef.json');

const fs = require('fs');

async function main() {
  // Initialise the provider to connect to the local node
  const provider = new WsProvider(process.argv[3]);

  // Create the API and wait until ready (optional provider passed through)
  const api = await ApiPromise.create({ provider, types });

  // Retrieve the upgrade key from the chain state
  const adminId = await api.query.sudo.key();

  // Find the actual keypair in the keyring (if this is a changed value, the key
  // needs to be added to the keyring before - this assumes we have defaults, i.e.
  // Alice as the key - and this already exists on the test keyring)
  const keyring = new Keyring({ type: 'sr25519' });
  const adminPair = keyring.addFromUri(process.argv[2]);

  // Retrieve the runtime to upgrade
  const code = fs
    .readFileSync('./target/release/wbuild/node-polkadex-runtime/node_polkadex_runtime.compact.wasm')
    .toString('hex');
  const proposal =
    api.tx.system && api.tx.system.setCode
      ? api.tx.system.setCode(`0x${code}`) // For newer versions of Substrate
      : api.tx.consensus.setCode(`0x${code}`); // For previous versions

  console.log(`Upgrading from ${adminId}, ${code.length / 2} bytes`);

  // Perform the actual chain upgrade via the sudo module
  api.tx.sudo.sudoUncheckedWeight(proposal, 0).signAndSend(adminPair, ({ events = [], status }) => {
    console.log('Proposal status:', status.type);

    if (status.isInBlock) {
      console.error('You have just upgraded your chain');

      console.log('Included at block hash', status.asInBlock.toHex());
      console.log('Events:');

      console.log(JSON.stringify(events, null, 2));

      process.exit(0);
    } else if (status.isFinalized) {
      console.log('Finalized block hash', status.asFinalized.toHex());

      process.exit(0);
    }
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(-1);
});
