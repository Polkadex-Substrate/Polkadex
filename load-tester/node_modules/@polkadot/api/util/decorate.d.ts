import type { AnyFunction } from '@polkadot/types/types';
import type { ApiTypes, DecorateMethod, MethodResult } from '../types';
declare type AnyDerive = Record<string, Record<string, AnyFunction>>;
declare type DeriveSection<ApiType extends ApiTypes, Section extends Record<string, AnyFunction>> = {
    [MethodName in keyof Section]: MethodResult<ApiType, Section[MethodName]>;
};
export declare type DeriveAllSections<ApiType extends ApiTypes, AllSections extends AnyDerive> = {
    [SectionName in keyof AllSections]: DeriveSection<ApiType, AllSections[SectionName]>;
};
/**
 * This is a section decorator which keeps all type information.
 */
export declare function decorateSections<ApiType extends ApiTypes, AllSections extends AnyDerive>(allSections: AllSections, decorateMethod: DecorateMethod<ApiType>): DeriveAllSections<ApiType, AllSections>;
export {};
