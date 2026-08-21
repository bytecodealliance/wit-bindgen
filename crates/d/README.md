# `wit-bindgen` D Bindings Generator

This tool generates [D](https://dlang.org) bindings for a chosen WIT world.

## Usage

To generate bindings with this crate, issue the `d` subcommand to `wit-bindgen`:

```bash
$ wit-bindgen d [OPTIONS] <WIT>
```

See the output of `wit-bindgen help d` for available options.

## Output Structure

Running `wit-bindgen d` on a WIT world will produce a full package structure mirroring the organization of the WIT world and the interfaces it uses. The default output directory and root package are both `wit`.

`wit.common` holds some definitions used across the generated bindings (for e.g. `result`, `option`, `list`, etc.).

The generated file for the world will by default, `public import` all relavent definitions. More selective imports can be made by importing the appropriate modules corresponding to particular interfaces.

The module for the world `foo:bar/world` will be placed in `wit/foo/bar/world/package.d`. Anonymous interface import and exports for the world will emitted in `wit/foo/bar/world/{imports|exports}`.

Top-level interfaces will be output similarly to worlds. `foo:bar/interface` gets split across multiple modules in `wit/foo/bar/interface/*.d`. Type definitions that are agnostic to import or export (any records, tuples, lists, etc. that do not contain resource handles) will go in `/common.d`. Function imports and import-specific types go in `/imports.d`, and exports go in `/exports.d`

## Memory/Resource Management

Currently, all imports take in parameters as "borrowing" and return memory "owning", with some caveats due to resource handles.

Memory is managed on the C `malloc` heap. This assumption can be used in some cases to reduce copies when unecessary.

When giving parameters to imports, any `list` or `string` memory is NOT freed, and the caller retains ownership/responibility for it. However, the ownership rules of WIT mean that any owning resource handles are invalidated. The bindings will not automatically "consume" these for you, and you will have to zero them out (default init) yourself after the call to prevent double-free.

When receiving parameters to exports, `list` and `string` memory is automatically freed. You must copy this memory elsewhere to retain it past the end of the call. Resource handles are not dropped however, and BOTH owning and borrowing handles (e.g. `Res` and `Res.Borrow`) must be dropped. To drop a handle, use `witDrop`.

When receiving values from imports, you are in charge of freeing the memory and dropping resource handles after you are finished with it/them. `witFree` deeply frees all such memory, and `witDrop` is also deep.

When return values from exports, all memory must be on the WIT/C heap, so it can be reliably `free`d by the bindings. `witClone` deeply copies all `list`s and `string`s.

`scope(exit)` is a valuable tool in helping keep track of this.

In the future, changes may be made to help make this more automatic.

## Examples

It is recommended to peruse `tests/runtime` to find concrete examples of how to use the bindings.
