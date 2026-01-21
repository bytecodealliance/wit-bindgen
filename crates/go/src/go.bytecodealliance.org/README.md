# Go Import Paths

This directory hosts the configuration for [Go module import paths](https://golang.org/cmd/go/#hdr-Remote_import_paths) under the `go.bytecodealliance.org` domain. Each unique Go module (with a separate `go.mod` file) must have a corresponding directory and `index.html` file linking to this repository (or another respository, see below).

Packages within a named module, e.g. `go.bytecodealliance.org/cm` will automatically be found.

## Creating a New Module

### In This Repository

To create a new Go module `go.bytecodealliance.org/gopher` **in this repository**:

1. Create a new `gopher` directory in the root of this repository.
2. In the `gopher` directory, run `go mod init go.bytecodealliance.org/gopher`.
3. Create a new `docs/gopher` directory.
4. Create a file `docs/gopher/index.html` with the following content:

	```html
	<html>
		<head>
			<meta name="go-import" content="go.bytecodealliance.org/gopher git https://github.com/bytecodealliance/go-modules" />
		</head>
	</html>
	```
5. Commit these changes and submit a Pull Request to this repository.

### In Another Repository

To create a new Go module `go.bytecodealliance.org/wasmtime` **in another repository** (e.g. github.com/bytecodealliance/wasmtime-go).

1. In the `wasmtime-go` repository, run `go mod init go.bytecodealliance.org/wasmtime`.
2. In this repository, create a new `docs/wasmtime` directory.
3. Create a file `docs/wasmtime/index.html` with the following content:

	```html
	<html>
		<head>
			<meta name="go-import" content="go.bytecodealliance.org/wasmtime git https://github.com/bytecodealliance/wasmtime-go" />
		</head>
	</html>
	```
4. Commit these changes and submit a Pull Request to this repository.


**Note**: Go import paths must be unique and not shadow existing package names in this repository. For example: Creating an import path `go.bytecodealliance.org/cm` would shadow package `cm` in this repository.
