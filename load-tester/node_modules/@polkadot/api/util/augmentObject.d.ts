/**
 * Takes a decorated api section (e.g. api.tx) and augment it with the details. It does not override what is
 * already available, but rather just adds new missing ites into the result object.
 * @internal
 */
export declare function augmentObject(prefix: string | null, src: Record<string, Record<string, unknown>>, dst: Record<string, Record<string, unknown>>, fromEmpty?: boolean): Record<string, Record<string, any>>;
