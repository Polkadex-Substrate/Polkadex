import type { JsonRpcRequest, JsonRpcResponse } from '../types';
/** @internal */
export declare class RpcCoder {
    #private;
    decodeResponse(response: JsonRpcResponse): unknown;
    encodeJson(method: string, params: any | any[]): string;
    encodeObject(method: string, params: unknown[]): JsonRpcRequest;
    getId(): number;
    private _checkError;
}
