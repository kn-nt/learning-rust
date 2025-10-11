/* tslint:disable */
/* eslint-disable */
/**
 * # Steps for webgpu
 * ## General Setup
 * - Create canvas -> generate surface from canvas
 * - Create adapter -> use it to get device & queue
 *     - Adapter is abstraction to represent physical or virtual GPU
 *     - Configure surface
 * ## Shaders
 * - Create shader source -> module
 * - Create pipeline layout -> use for render pipeline
 *     - Pipeline Layout is for defining how shaders access resources
 *     - Render Pipeline defines which shaders, resources, data, etc. to use and how to output data
 * ## Render
 * - Create render pass descriptor
 *     - Render Pass Descriptor describes how one Render Pass should work
 *         - Render Pass is a batch of drawing commands
 * - Create encoder
 *     - Encoder records GPU commands into a command buffer for the GPU to queue and work on
 * ## Drawing
 * - Using Render Pass, set pipeline & issue draw command
 * ## Submit
 * - Submit work using queue
 */
export function main(): Promise<void>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly main: () => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly closure784_externref_shim: (a: number, b: number, c: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
