/**
 * # Variants
 * 
 * ## `"js"`
 * 
 * ## `"rust"`
 * 
 * ## `"java"`
 * 
 * ## `"c"`
 * 
 * ## `"markdown"`
 */
export type Lang = 'js' | 'rust' | 'java' | 'c' | 'markdown';
export interface Options {
  rustUnchecked: boolean,
  jsCompat: boolean,
  jsInstantiation: boolean,
}
export type Files = [string, string][];
export namespace Demo {
  export function render(lang: Lang, wit: string, options: Options): Files;
}
