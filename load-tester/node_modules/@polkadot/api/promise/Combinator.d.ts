import type { Callback } from '@polkadot/types/types';
import type { UnsubscribePromise } from '../types';
export declare type CombinatorCallback<T extends any[]> = Callback<T>;
export interface CombinatorFunction {
    (cb: Callback<any>): UnsubscribePromise;
}
export declare class Combinator<T extends any[] = any[]> {
    #private;
    constructor(fns: (CombinatorFunction | [CombinatorFunction, ...any[]])[], callback: CombinatorCallback<T>);
    protected _allHasFired(): boolean;
    protected _createCallback(index: number): (value: any) => void;
    protected _triggerUpdate(): void;
    unsubscribe(): void;
}
