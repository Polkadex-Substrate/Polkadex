# IPFS Pallet

The goal of IPFS pallet is the provide a means to securely store the balance snapshots of users in Polkadex exchange. 
The pallet will accept IPFS Content Identifiers provided by the enclave, which are accepted until verified by validators of the network.

On new CID, the validators will try to sync that CID, and verify the integrity of data, once verified, once of the next block producer
will insert an inherent call approving the latest cid. 


In the event of a emergency shutdown of exchange either by On-chain governance or by other automatic means in this pallet. It will stop further 
deposits and withdrawals from Polkadex exchange, and enables the users to claim back their funds using the latest approvid cid.


NOTE: This pallet is neither audited nor complete.