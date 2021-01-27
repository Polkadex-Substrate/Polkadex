import type { Registry } from '../types';
import { Enum } from '../codec/Enum';
export declare class GenericMultiAddress extends Enum {
    constructor(registry: Registry, value?: unknown);
    /**
     * @description Returns the string representation of the value
     */
    toString(): string;
}
