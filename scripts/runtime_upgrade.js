const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const types = require('../typedef.json');

const fs = require('fs');

async function main() {
  const provider = new WsProvider(process.argv[3]);

  const api = await ApiPromise.create({ provider, types });

  const adminId = await api.query.sudo.key();

  const keyring = new Keyring({ type: 'sr25519' });
  const adminPair = keyring.addFromUri(process.argv[2]);

  const code = fs
    .readFileSync('./target/release/wbuild/node-polkadex-runtime/node_polkadex_runtime.compact.wasm')
    .toString('hex');
  const proposal =
    api.tx.system && api.tx.system.setCode ? api.tx.system.setCode(`0x${code}`) : api.tx.consensus.setCode(`0x${code}`);

  console.log(`Upgrading from ${adminId}, ${code.length / 2} bytes`);

  api.tx.sudo.sudoUncheckedWeight(proposal, 0).signAndSend(adminPair, ({ events = [], status }) => {
    console.log('Proposal status:', status.type);

    if (status.isInBlock) {
      console.log('You have just upgraded your chain');

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
