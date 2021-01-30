import type { AnyFunction, AnyTuple, CallBase } from '@polkadot/types/types';
import type { SubmittableExtrinsic } from '../submittable/types';
import type { ApiTypes } from './base';
export interface AugmentedSubmittables<ApiType extends ApiTypes> {
}
export declare type AugmentedSubmittable<T extends AnyFunction, A extends AnyTuple = AnyTuple> = T & CallBase<A>;
export interface SubmittableExtrinsicFunction<ApiType extends ApiTypes, A extends AnyTuple = AnyTuple> extends CallBase<A> {
    (...params: any[]): SubmittableExtrinsic<ApiType>;
}
export interface SubmittableModuleExtrinsics<ApiType extends ApiTypes> {
    [index: string]: SubmittableExtrinsicFunction<ApiType>;
}
