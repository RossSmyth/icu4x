// generated by diplomat-tool
import type { CodePointSetData } from "./CodePointSetData"
import type { pointer, char } from "./diplomat-runtime.d.ts";


/** See the [Rust documentation for `CodePointInversionListBuilder`](https://docs.rs/icu/latest/icu/collections/codepointinvlist/struct.CodePointInversionListBuilder.html) for more information.
*/
export class CodePointSetBuilder {
    

    get ffiValue(): pointer;


    static create(): CodePointSetBuilder;

    build(): CodePointSetData;

    complement(): void;

    get isEmpty(): boolean;

    addChar(ch: char): void;

    addInclusiveRange(start: char, end: char): void;

    addSet(data: CodePointSetData): void;

    removeChar(ch: char): void;

    removeInclusiveRange(start: char, end: char): void;

    removeSet(data: CodePointSetData): void;

    retainChar(ch: char): void;

    retainInclusiveRange(start: char, end: char): void;

    retainSet(data: CodePointSetData): void;

    complementChar(ch: char): void;

    complementInclusiveRange(start: char, end: char): void;

    complementSet(data: CodePointSetData): void;

    

}