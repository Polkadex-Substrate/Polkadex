import type { EventRecord, ExtrinsicStatus, H256, SignedBlock } from '@polkadot/types/interfaces';
export declare function filterEvents(extHash: H256, { block: { extrinsics, header } }: SignedBlock, allEvents: EventRecord[], status: ExtrinsicStatus): EventRecord[] | undefined;
