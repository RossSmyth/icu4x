// generated by diplomat-tool
import type { DataProvider } from "./DataProvider"
import type { DisplayNamesOptions } from "./DisplayNamesOptions"
import type { Locale } from "./Locale"
import type { pointer, char } from "./diplomat-runtime.d.ts";


/** See the [Rust documentation for `LocaleDisplayNamesFormatter`](https://docs.rs/icu/latest/icu/displaynames/struct.LocaleDisplayNamesFormatter.html) for more information.
*/
export class LocaleDisplayNamesFormatter {
    

    get ffiValue(): pointer;


    static create(provider: DataProvider, locale: Locale, options: DisplayNamesOptions): LocaleDisplayNamesFormatter;

    of(locale: Locale): string;

    

}