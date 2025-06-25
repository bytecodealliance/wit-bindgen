# `wit-bindgen` C and C++ Bindings Generator

This tool generates C bindings for a chosen WIT world that are also compatible with C++. 

## Usage

To generate bindings with this crate, issue the `c` subcommand to `wit-bindgen`:

```bash
$ wit-bindgen c [OPTIONS] <WIT>
```

See the output of `wit-bindgen help c` for available options.

This command will generate either two or three files, depending on command line options:

- `<world>.h`: Header file that declares the available bindings. 
- `<world>.c`: Source file that implements bindings for imports, helper functions, and wrappers around exported functions.
- `<world>_component_type.o`: An object file that contains type information for the world that bindings were generated for. This file is not generated if the `--no-object-file` command line flag is passed.

`<world>` in each of these is the name of the WIT world, converted to snake case (e.g. `i-am-a-component` generates `i_am_a_component.h`)

## Generated Bindings

### Memory Ownership

In this section, *your component* refers to the component for which you are generating bindings.

In general, your component is responsible for allocating memory for data it produces and freeing memory for data it consumes. `wit-bindgen` automatically generates `*_free` functions for types that require allocation (e.g. strings, lists, records with fields that require allocation) to properly deallocate their memory, including any nested allocated types.

There are additional ownership considerations for `string` and `resource`. See the [Strings](#strings) and [Resources](#resources) sections for details.

Ownership rules differ for imported functions and exported functions, so we'll tackle these in turn.

#### Imported functions

Imported functions are functions that are implemented in another component and that your component calls.

_Ownership rules_:

- **Arguments**: Your component is responsible for allocating memory for the input arguments and freeing this memory when it is no longer required. Your input data will not be modified or freed when you call the imported function; the data will be copied into the linear memory of the component that implements the function.
- **Return values**: Your component receives ownership of data returned by imported functions and is responsible for freeing it.

Here is an example of an imported function that requires allocations for both input parameters and the return value:

```wit
package cat:registry;

interface cat-registry-api {
    record cat {
        name: string,
        nicknames: list<string>,
    }
    get-cat-by-name: func(name: string) -> option<cat>;
}

world cat-registry-user {
    import cat-registry-api;
    export run: func();
}
```

Here is a possible implementation of the `run` export that calls `get-cat-by-name` with `Poptart` as an argument without leaking memory:

```c
void exports_cat_registry_user_run(void) {
  cat_registry_user_string_t name;
  // Allocates memory for the string and sets it to "Poptart"
  // (This could call cat_registry_user_string_set to avoid allocating,
  // the allocation is just for demonstration purposes)
  cat_registry_user_string_dup(&name, "Poptart");
  cat_registry_cat_registry_api_cat_t cat;
  bool got_cat = cat_registry_cat_registry_api_get_cat_by_name(&name, &cat);

  // Do something with the returned cat
  // ...

  // Frees the string we allocated for the argument to the import
  cat_registry_user_string_free(&name);
  // Frees the memory allocated for the import's return value
  if (got_cat) {
    cat_registry_cat_registry_api_cat_free(&cat);
  }
}
```

Note that we free the memory allocated for the import's argument as well as its return value.

#### Exported functions

Exported functions are functions that your component implements and makes available to other components.

_Ownership rules_:

- **Arguments**: Your component receives ownership of all arguments passed to the exported function and is responsible for freeing them.
- **Return values**: Your component is responsible for allocating the memory for return values. It is not responsible for freeing them: `wit-bindgen` automatically generates cleanup functions named `*_post_return` which will be called by the component who imports your function. This generated function assumes that all `string`s and `list`s in the return value are dynamically allocated.
- **Resource borrows**: There are special requirements for cleaning up borrows of imported resources received as parameters; see the [Imported resources](#imported-resources) section for more details. 

Here is an example of an exported function that requires allocations for both input parameters and the return value:

```wit
package cat:registry;

interface cat-registry-api {
    record cat {
        name: string,
        nicknames: list<string>,
    }
    get-cat-by-name: func(name: string) -> option<cat>;
}

world cat-registry {
    export cat-registry-api;
}
```

Here is a possible implementation of the `get-cat-by-name` export that correctly manages the memory of the parameter and return value:

```c
bool exports_cat_registry_cat_registry_api_get_cat_by_name(
    cat_registry_string_t *name,
    exports_cat_registry_cat_registry_api_cat_t *ret) {
  // We only support the name "Poptart" for this example
  bool found = strncmp((const char *)name->ptr, "Poptart", name->len) == 0;

  if (found) {
    // Allocate memory for the return name and copy the name in
    cat_registry_string_dup(&ret->name, "Poptart");

    // Allocate memory for the nicknames list
    ret->nicknames.ptr =
      (cat_registry_string_t *)malloc(2 * sizeof(cat_registry_string_t));
    ret->nicknames.len = 2;
    // Allocate memory for each nickname and copy the strings in
    cat_registry_string_dup(&ret->nicknames.ptr[0], "Poppy");
    cat_registry_string_dup(&ret->nicknames.ptr[1], "Popster");
  }

  // We own the memory for the input parameter, so we must free it
  cat_registry_string_free(name);

  // We allocated memory for the return value, but freeing it is handled by the generated bindings
  return found;
}
```

Importantly, the generated `*_post_return` function that frees the return value assumes that lists and strings in the return value reference dynamically-allocated memory that can be freed with `free`. As such, if you call `*_string_set` with a string literal to set one of the fields of the return value, the bindings will try to free the string literal. Unless you want to replace the generated `*_post_return` function with a version that only frees some of the data, always dynamically allocate the contents of your strings and lists in your return values. The `*_post_return` functions are defined as [weak symbols](https://en.wikipedia.org/wiki/Weak_symbol) so that you can simply define your own versions to override the defaults and they will be selected by the linker.

### Type Mappings

#### Primitive types

| WIT Type | C Type     |
|----------|------------|
| `bool`   | `bool`     |
| `u8`     | `uint8_t`  |
| `u16`    | `uint16_t` |
| `u32`    | `uint32_t` |
| `u64`    | `uint64_t` |
| `s8`     | `int8_t`   |
| `s16`    | `int16_t`  |
| `s32`    | `int32_t`  |
| `s64`    | `int64_t`  |
| `f32`    | `float`    |
| `f64`    | `double`   |
| `char`   | `uint32_t` |

Note that `uint32_t` is used to represent WIT's `char` type because WIT `char`s are [Unicode Scalar Values](https://www.unicode.org/glossary/#unicode_scalar_value).

#### Strings

The representation of strings depends on the `--string-encoding` command line flag. The two supported options are `utf8` and `utf16`. In both cases, they are represented by a pointer to the string data along with their length in [Unicode code units](https://www.unicode.org/glossary/#code_unit):

```c
// UTF-8 version
typedef struct my_world_string_t {
    uint8_t *ptr;    // Pointer to string data
    size_t len;      // Length in code units
} my_world_string_t;

// UTF-16 version
typedef struct my_world_string_t {
    uint16_t *ptr;   // Pointer to string data
    size_t len;      // Length in code units
} my_world_string_t;
```

Alongside the string type, the following functions will be defined (assuming UTF-8 strings):

```c
// Sets the string `ret` to reference the input string `s` without copying it
void my_world_string_set(my_world_string_t *ret, const char8_t*s);

// Creates a copy of the input nul-terminated string `s` and
// stores it into the component model string `ret`.
void my_world_string_dup(my_world_string_t *ret, const char8_t*s);

// Deallocates the string pointed to by `ret`, deallocating
// the memory behind the string.
void my_world_string_free(my_world_string_t *ret);
```

For UTF-16 strings, those `char8_t*`s become `char16_t*`s and the following function is also supplied:

```c
// Returns the length of the UTF-16 string `s` in code units
size_t my_world_string_len(const char16_t* s);
```

If a component calls `*_string_set`, the component is responsible for freeing the string if it was dynamically allocated. For example:

```c
my_world_string_t static_string;
my_world_string_set(&static_string, "Don't free me please!");
// Use static_string
// ...
// Do *not* call my_world_string_free(&static_string);

my_world_string_t dynamic_string;
char8_t* my_string = get_string_somehow(); // calls malloc internally
my_world_string_set(&dynamic_string, my_string);
// Use dynamic_string
// ...
// Free the memory
my_world_string_free(&dynamic_string);
```

#### Lists

WIT `list`s are represented as structures containing a pointer and length:

```c
// list<u8>
typedef struct my_world_list_u8_t {
    uint8_t *ptr;
    size_t len;
} my_world_list_u8_t;
``` 

A helper function is generated for freeing `list`s:

```c
// Frees the memory associated with the list
void my_world_list_u8_free(my_world_list_u8_t *ptr);
```

#### Variants

WIT `variant`s are represented as [tagged unions](https://en.wikipedia.org/wiki/Tagged_union):

```c
// variant cat-toy { ball, teddy(string), wand }
typedef struct my_world_cat_toy_t {
  uint8_t tag;
  union {
    my_world_string_t     teddy;
  } val;
} my_world_cat_toy_t;

#define MY_WORLD_CAT_TOY_BALL 0
#define MY_WORLD_CAT_TOY_TEDDY 1
#define MY_WORLD_CAT_TOY_WAND 2
```

A helper function is generated for freeing `variant`s:

```c
void my_world_cat_toy_free(my_world_cat_toy_t *ptr);
```

#### Results

`result`s are specified with a union and `is_err` field:

```c
// result<string, u32>
typedef struct {
  bool is_err;
  union {
    my_world_string_t ok;
    uint32_t err;
  } val;
} my_world_result_string_u32_t;
```

If the `--no-sig-flattening` command line argument is passed, then bindings for functions that return `result<T,E>` take a single out parameter, which is consistent with other return types. Otherwise, the return value is _flattened_ by taking out parameters for `T` and `E` and returning `!is_err` as a boolean result. For example:

```wit
package my:example;
interface string-getter {
    type error = u32;
    get-string-by-index: func(index: u32) -> result<string, error>;
}
```

Depending on the command line options passed, this will generate one of these two functions:

```c
// --no-sig-flattening passed, the usual single out parameter is used
extern void
my_example_string_getter_get_string_by_index(
    uint32_t index, my_example_string_getter_result_string_error_t *ret);

// Flag not passed, the return value is flattened
extern bool
my_example_string_getter_get_string_by_index(
    uint32_t index, string_getter_user_string_t *ret, my_example_string_getter_error_t *err);
```

A helper function is generated for freeing `result`s:

```c
void my_world_result_string_u32_free(my_world_result_string_u32_t *ptr);
```

#### Options

`option<T>`s are specified with an `is_some` boolean member that discriminates whether a `T` value is contained:

```c
// option<string>
typedef struct my_world_option_string_t {
    bool is_some;
    my_world_string_t val;
} my_world_option_string_t;
```

If the `--no-sig-flattening` command line argument is passed, then bindings for functions that return `option<T>` take a single out parameter, which is consistent with other return types. Otherwise, the return value is _flattened_ by taking an out parameter for `T` and returning `is_some` as a boolean result. For example:

```wit
package my:example;
interface string-getter {
    get-string-by-index: func(index: u32) -> option<string>;
}
```

Depending on the command line options passed, this will generate one of these two functions:

```c
// --no-sig-flattening passed, the usual single out parameter is used
extern void my_example_string_getter_get_string_by_index(uint32_t index, string_getter_user_option_string_t *ret);

// Flag not passed, the return value is flattened
extern bool my_example_string_getter_get_string_by_index(uint32_t index, string_getter_user_string_t *ret);
```

A helper function is generated for freeing `option`s:

```c
void my_world_option_string_free(my_world_option_string_t *ptr);
```

#### Enums and Flags

Enums are mapped to typedefs for a sufficiently-wide integer backing type:

```c
// enum cat-breed { persian, siamese, ragdoll }
typedef uint8_t my_world_cat_breed_t;

#define MY_WORLD_CAT_BREED_PERSIAN 0
#define MY_WORLD_CAT_BREED_SIAMESE 1
#define MY_WORLD_CAT_BREED_RAGDOLL 2
```

Flags are similarly defined with bitfield constants:

```c
// flags cat-flags { is-fluffy, has-been-fed, is-sleepy }
typedef uint8_t my_world_cat_flags_t;

#define MY_WORLD_CAT_FLAGS_IS_FLUFFY (1 << 0)
#define MY_WORLD_CAT_FLAGS_HAS_BEEN_FED (1 << 1)
#define MY_WORLD_CAT_FLAGS_IS_SLEEPY (1 << 2)
```

#### Resources

WIT resources are represented differently for imports and exports, and each have memory ownership constraints that you must be mindful of.

The following WIT definition will be used in the following examples:

```wit
package cat:example;

// A cat registry that defines an opaque resource
interface registry-api {
    // The exporter of this resource will define how the cat is represented
    resource cat {
        get-name: func() -> string;
        get-nicknames: func() -> list<string>;
    }
    // Various functions that will demonstrate different uses of the exported resource
    adopt-cat: func(name: string) -> option<cat>;
    notify-adopted-cat-is-happy: func(cat: borrow<cat>);
    enroll-as-therapy-cat: func(cat: cat);

    // Setup and teardown functions for the registry, which will be called
    // by the adopter
    init: func();
    destroy: func();
}

// An adoption authority that adopters must notify when they adopt a cat
interface adoption-authority-api {
    use registry-api.{cat};
    // This will demonstrate exporting functions that borrow an imported resource
    notify-adoption: func(cat: borrow<cat>);
}

// The adoption authority imports the registry API to use the cat resource
world adoption-authority {
    import registry-api;
    export adoption-authority-api;
}

world registry {
    export registry-api;
}

// The adopter is the main component in this example.
// It is responsible for initializing the registry,
// adopting cats, and notifying the adoption authority.
world adopter {
    import adoption-authority-api;
    import registry-api;
    // Entry point that can be called from the command line
    export wasi:cli/run@0.2.6;
}
```

##### Imported resources

Imported resources are represented as opaque handles with helper functions for resource management. 

The following bindings will be generated for imports of `cat`:

```c
// An owning handle to a cat
typedef struct cat_example_registry_api_own_cat_t {
  int32_t __handle;
} cat_example_registry_api_own_cat_t;

// A borrowing handle to a cat
typedef struct cat_example_registry_api_borrow_cat_t {
  int32_t __handle;
} cat_example_registry_api_borrow_cat_t;

// Functions to call the methods of the cat resource
extern void
cat_example_registry_api_method_cat_get_name(
    cat_example_registry_api_borrow_cat_t self, adoption_authority_string_t *ret);
extern void
cat_example_registry_api_method_cat_get_nicknames(
    cat_example_registry_api_borrow_cat_t self, adoption_authority_list_string_t *ret);

// Drop an owning handle to a cat
extern void
cat_example_registry_api_cat_drop_own(
    cat_example_registry_api_own_cat_t handle);

// Drop a borrowing handle to a cat
// Only generated if autodropping borrows is turned off (described below)
extern void
cat_example_registry_api_cat_drop_borrow(
    cat_example_registry_api_borrow_cat_t handle);

// Retrieve a borrowing handle to a cat from an owning handle
extern cat_example_registry_api_borrow_cat_t
cat_example_registry_api_borrow_cat(
    cat_example_registry_api_own_cat_t handle);

```

A component may _borrow_ or _own_ an imported resource. This is communicated in WIT by whether the resource type is wrapped in `borrow<...>` or not. The importing component is responsible for dropping any owning references to imported resources using the generated `*_drop_own` function. 

The importing component's responsibility for dropping borrows of imported resources depends on the command line arguments passed to `wit-bindgen` and whether it obtained the borrow by calling a generated `*_borrow_*` function on an owned resource, or by receiving it as an argument to one of its exported functions. The relevant cases are:

- If the borrow was passed as an argument to one of the importing component's exports:
    - If the bindings were generated with `--autodrop-borrows=yes`, then borrowed handles will be automatically dropped when the exported function returns; no additional action is required by importing component
    - If the bindings were generated with `--autodrop-borrow=no`, or this command line option was not supplied, then the importing component must drop the borrowed handle by calling the generated `*_drop_borrow` function.
- If the component obtained the borrow by calling `*_borrow*_` on an owned resource, it must not call `*_drop_borrow`

Here is a potential implementation of the `run` export that correctly handles the lifetime of the owned cat resource retrieved from `get-cat-by-name`:

```c
bool exports_wasi_cli_run_run(void) {
  cat_example_registry_api_init();

  adopter_string_t name;
  // Allocates memory for the string and sets it to "Poptart"
  adopter_string_dup(&name, "Poptart");

  // If this function returns true, it means we successfully adopted the cat
  // and `cat` will contain the adopted cat handle. We are responsible for
  // dropping the handle to this cat when we are done with it.
  cat_example_registry_api_own_cat_t cat;
  bool got_cat = cat_example_registry_api_adopt_cat(&name, &cat);

  // Free the memory allocated for import argument
  adopter_string_free(&name);

  if (got_cat) {
    // When taking a borrow of a handle we own, we must not drop
    // the borrowed handle.
    cat_example_adoption_authority_api_notify_adoption(
        cat_example_registry_api_borrow_cat(cat));

    // We must drop the owning handle when we are done with it
    cat_example_registry_api_cat_drop_own(cat);
  }

  cat_example_registry_api_destroy();
  return got_cat;
}
```

A correct implementation of `notify-adoption` depends on whether autodropping borrows is enabled. Here is a possible implementation for both cases:

```c
// Autodropping borrows enabled
void exports_cat_example_adoption_authority_api_notify_adoption(
    exports_cat_example_adoption_authority_api_borrow_cat_t cat) {
  // Do something with the borrowed cat
  // ...

  // Do *not* drop the borrow, it is done automatically
}

// Autodropping borrows disabled
void exports_cat_example_adoption_authority_api_notify_adoption(
    exports_cat_example_adoption_authority_api_borrow_cat_t cat) {
  // Do something with the borrowed cat
  // ...

  // Manually drop the borrowed handle
  cat_example_registry_api_cat_drop_borrow(cat);
}
```

##### Exported resources

Exported resources are represented by an opaque handle to a custom internal representation defined by the component.

The following bindings will be generated for exports of `cat`:

```c
// An owning opaque handle for a cat
typedef struct exports_cat_example_registry_api_own_cat_t {
  int32_t __handle;
} exports_cat_example_registry_api_own_cat_t;

// The internal representation of the resource - must be implemented by the exporting component
typedef struct exports_cat_example_registry_api_cat_t
    exports_cat_example_registry_api_cat_t;

// Borrows are represented as a pointer to the internal representation rather than an opaque handle
typedef exports_cat_example_registry_api_cat_t*
    exports_cat_example_registry_api_borrow_cat_t;

// Declarations for the resource methods that the exporting component must implement
void exports_cat_example_registry_api_method_cat_get_name(
    exports_cat_example_registry_api_borrow_cat_t self, registry_string_t *ret);
void exports_cat_example_registry_api_method_cat_get_nicknames(
    exports_cat_example_registry_api_borrow_cat_t self, registry_list_string_t *ret);

// Drop an owning handle to a cat
extern void
exports_cat_example_registry_api_cat_drop_own(
    exports_cat_example_registry_api_own_cat_t handle);

// Create an owning opaque handle to a cat from an internal representation
extern exports_cat_example_registry_api_own_cat_t
exports_cat_example_registry_api_cat_new(
    exports_cat_example_registry_api_cat_t *rep);

// Get the internal representation of a cat from an owning opaque handle
extern exports_cat_example_registry_api_cat_t*
exports_cat_example_registry_api_cat_rep(
    exports_cat_example_registry_api_own_cat_t handle);

// Destroy a cat resource - must be implemented by the exporting component
void exports_cat_example_registry_api_cat_destructor(
    exports_cat_example_registry_api_cat_t *rep);
```

Here are possible implementations for the functions and type that must be implemented by the component:

```c
struct exports_cat_example_registry_api_cat_t {
  registry_string_t name;
  registry_list_string_t nicknames;
};

// Custom destructor that frees the memory of a cat resource
void exports_cat_example_registry_api_cat_destructor(
    exports_cat_example_registry_api_cat_t *arg) {
  registry_string_free(&arg->name);
  registry_list_string_free(&arg->nicknames);
  free(arg);
}

// Implementation for the get-name method
void exports_cat_example_registry_api_method_cat_get_name(
    exports_cat_example_registry_api_borrow_cat_t self,
    registry_string_t *ret) {
  registry_string_dup(ret, (const char *)self->name.ptr);
}

// Implementation for the get-nicknames method
void exports_cat_example_registry_api_method_cat_get_nicknames(
    exports_cat_example_registry_api_borrow_cat_t self,
    registry_list_string_t *ret) {
  ret->len = self->nicknames.len;
  ret->ptr =
      (registry_string_t *)malloc(ret->len * sizeof(registry_string_t));
  for (size_t i = 0; i < ret->len; i++) {
    registry_string_dup(&ret->ptr[i],
                            (const char *)self->nicknames.ptr[i].ptr);
  }
}
```

A component may _borrow_ or _own_ an exported resource. This is communicated in WIT by whether the resource type is wrapped in `borrow<...>` or not. The exporting component is responsible for dropping any owning references to resources that it creates through calls to `*_new` using the generated `*_drop_own` function. Components must not drop borrowing handles to resources that they export.

The implementations of the destructor and the `cat` methods would be the same as the above example. Here are possible implementations of the other functions:

```c
// Scary scary global state
// This tracks all the cats that are registered,
// and will be filled in by the `init` function.
exports_cat_example_registry_api_own_cat_t *g_cats;
size_t g_cat_count = 0;
const size_t MAX_CAT_COUNT = 32;

bool exports_cat_example_registry_api_adopt_cat(
    registry_string_t *name,
    exports_cat_example_registry_api_own_cat_t *ret) {
  bool found = false;
  for (size_t i = 0; i < g_cat_count; i++) {
    exports_cat_example_registry_api_own_cat_t *cat = &g_cats[i];
    // Get the internal representation for the owning handle
    exports_cat_example_registry_api_cat_t *cat_rep =
        exports_cat_example_registry_api_cat_rep(*cat);
    // If the name matches, we found the cat to adopt
    if (cat_rep->name.len == name->len &&
        memcmp(cat_rep->name.ptr, name->ptr, name->len) == 0) {
      *ret = *cat;
      found = true;

      // Shuffle down cats to remove this one from the registry
      for (size_t j = i; j < g_cat_count - 1; j++) {
        g_cats[j] = g_cats[j + 1];
      }
      g_cat_count--;

      break;
    }
  }
  // Free the memory allocated for the argument
  registry_string_free(name);

  // After this function returns, the caller owns any cat handle
  // that was returned in `ret`, so we should not drop it here
  return found;
}

void exports_cat_example_registry_api_notify_adopted_cat_is_happy(
    exports_cat_example_registry_api_borrow_cat_t cat) {
  // Do something with this information
  // ...

  // No need to drop the handle here
}
void exports_cat_example_registry_api_enroll_as_therapy_cat(
    exports_cat_example_registry_api_own_cat_t cat) {
  // Notify the cat therapy service

  // We're now done with this handle, so we drop it, which will call the
  // destructor and free the memory allocated for the cat.
  exports_cat_example_registry_api_cat_drop_own(cat);
}

void exports_cat_example_registry_api_init(void) {
  // Create a cat to be added to the registry.
  // The internal representation must live at least as long 
  // as the owning handle, so we dynamically allocate it.
  exports_cat_example_registry_api_cat_t *poptart =
      malloc(sizeof(exports_cat_example_registry_api_cat_t));
  registry_string_dup(&poptart->name, "Poptart");
  poptart->nicknames.len = 2;
  poptart->nicknames.ptr = (registry_string_t *)malloc(
      poptart->nicknames.len * sizeof(registry_string_t));
  registry_string_dup(&poptart->nicknames.ptr[0], "Poppy");
  registry_string_dup(&poptart->nicknames.ptr[1], "Popster");

  // Initialize the registry with the created cat
  g_cat_count = 1;
  g_cats = malloc(MAX_CAT_COUNT *
                  sizeof(exports_cat_example_registry_api_own_cat_t));

  // This creates an owning handle for the cat, which we will need to either
  // drop when we're done or pass ownership to another component.                  
  g_cats[0] = exports_cat_example_registry_api_cat_new(poptart);
}

void exports_cat_example_registry_api_destroy(void) {
   // Drop all remaining owning cat handles to free their resources
  for (size_t i = 0; i < g_cat_count; i++) {
    exports_cat_example_registry_api_cat_drop_own(g_cats[i]);
  }
  free(g_cats);
}
```

### C++ compatibilitity

All types and functions are wrapped in the following block, which gives the symbols C linkage when compiling in C++ mode:

```cpp
#ifdef __cplusplus
extern "C" {
#endif

// bindings

#ifdef __cplusplus
}
#endif
```



