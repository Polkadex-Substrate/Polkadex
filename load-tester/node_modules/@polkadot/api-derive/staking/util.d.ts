import type { EraIndex } from '@polkadot/types/interfaces';
export declare function filterEras<T extends {
    era: EraIndex;
}>(eras: EraIndex[], list: T[]): EraIndex[];
