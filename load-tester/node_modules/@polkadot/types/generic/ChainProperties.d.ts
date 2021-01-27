import type { Registry } from '../types';
import { Json } from '../codec/Json';
import { Option } from '../codec/Option';
import { Vec } from '../codec/Vec';
import { Text } from '../primitive/Text';
import { u32 } from '../primitive/U32';
export declare class GenericChainProperties extends Json {
    constructor(registry: Registry, value?: Map<string, unknown> | Record<string, unknown> | null);
    /**
     * @description The chain ss58Format
     */
    get ss58Format(): Option<u32>;
    /**
     * @description The decimals for each of the tokens
     */
    get tokenDecimals(): Option<Vec<u32>>;
    /**
     * @description The symbols for the tokens
     */
    get tokenSymbol(): Option<Vec<Text>>;
}
