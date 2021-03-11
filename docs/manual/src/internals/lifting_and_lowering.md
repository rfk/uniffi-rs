# Lifting, Lowering and Serialization

UniFFI is able to transfer rich data types back-and-forth between the Rust
code and the foreign-language code via a process we refer to as "lowering"
and "lifting".

Recall that UniFFI interoperates between different languages by defining
a C-style FFI layer which operates in terms of primitive data types and
plain functions. To transfer data from one side of this layer to the other,
the sending side "***lowers***" the data from a language-specific data type
into one of the primitive types supported by the FFI-layer functions, and the
receiving side "***lifts***" that primitive type into its own language-specific
data type.

Lifting and lowering simple types such as integers is done by directly casting the
value to and from an appropriate type. For complex types such as optionals and
records we currently implement lifting and lowering by serializing into a byte
buffer, but this is an implementation detail that may change in future. (See
[ADR-0002](/docs/adr/0002-serialize-complex-datatypes.md) for the reasoning
behind this choice).

As a concrete example, consider this interface for accumulating a list of integers:

```rust
#[uniffi_macros::declare_interface]
mod example {
   pub fn add_to_list(item: i32) -> Vec<i32> { .. }
}
```

Calling this function from foreign language code involves the following steps:

1. The user-provided calling code invokes the `add_to_list` function that is exposed by the
   UniFFI-generated foreign language bindings, passing `item` as an appropriate language-native
   integer.
2. The foreign language bindings ***lower*** each argument to a function call into
   something that can be passed over the C-style FFI. Since the `item` argument is a plain integer,
   it is lowered by casting to an `int32_t`.
3. The foreign language bindings pass the lowered arguments to a C FFI function named
   like `example_XYZ_add_to_list` that is exposed by the UniFFI-generated Rust scaffolding.
4. The Rust scaffolding ***lifts*** each argument received over the FFI into a native
   Rust type. Since `item` is a plain integer it is lifted by casting to a Rust `i32`.
5. The Rust scaffolding passes the lifted arguments to the user-provided Rust code for
   the `add_to_list` function, which returns a `Vec<i32>`.
6. The Rust scaffolding now needs to ***lower*** the return value in order to pass it back
   to the foreign language code. Since this is a complex data type, it is lowered by serializing
   the values into a byte buffer and returning the buffer pointer and length from the
   FFI function.
7. The foreign language bindings receive the return value and need to ***lift*** it into an
   appropriate native data type. Since it is a complex data type, it is lifted by deserializing
   from the returned byte buffer into a language-native list of integers.

## Lowered Types

| Rust Type | Representation in the C FFI |
|----------|-----------------------------|
| `i8`/`i16`/`i32`/`i64` | `int8_t`/`int16_t`/`int32_t`/`int64_t` |
| `u8`/`u16`/`u32`/`u64` | `uint8_t`/`uint16_t`/`uint32_t`/`uint64_t` |
| `f32` | `float` |
| `f64` | `double` |
| `bool` | `int8_t`, either `0` or `1` |
| `String` | `RustBuffer` struct pointing to utf8 bytes |
| `Option<T>` | `RustBuffer` struct pointing to serialized bytes |
| `Vec<T>` | `RustBuffer` struct pointing to serialized bytes |
| `HashMap<String, T>` | `RustBuffer` struct pointing to serialized bytes |
| `enum` | `RustBuffer` struct pointing to serialized bytes |
| Record `struct` | `RustBuffer` struct pointing to serialized bytes |
| Object `struct` | `uint64_t` opaque integer handle |


## Serialization Format

When serializing complex data types into a byte buffer, UniFFI uses an
ad-hoc fixed-width format which is designed mainly for simplicity.
The details of this format are internal only and may change between versions of UniFFI.

| UDL Type | Representation in serialized bytes |
|----------|-----------------------------|
| `i8`/`i16`/`i32`/`i64` | Fixed-width 1/2/4/8-byte signed integer, big-endian|
| `u8`/`u16`/`u32`/`u64` | Fixed-width 1/2/4/8-byte unsigned integer, big-endian |
| `f32` | Fixed-width 4-byte float, big-endian |
| `f64` | Fixed-width 8-byte double, big-endian |
| `bool` | Fixed-width 1-byte signed integer, either `0` or `1` |
| `String` | Serialized `i32` length followed by utf-8 string bytes; no trailing null |
| `Option<T>` | If null, serialized `bool` false; if non-null, serialized `bool` true followed by serialized `T` |
| `Vec<T>` | Serialized `i32` item count followed by serialized items; each item is a serialized `T` |
| `HashMap<String, T>` | Serialized `i32` item count followed by serialized items; each item is a serialized `string` followed by a serialized `T` |
| `enum` | Serialized `i32` indicating variant, numbered in declaration order starting from 1, followed by the serialized values of the variant's fields in declaration order |
| Record `struct` | The serialized value of each field, in declaration order |
| Object `struct` | *Cannot currently be serialized* |

Note that length fields in this format are serialized as *signed* integers
despite the fact that they will always be non-negative. This is to help
ease compatibility with JVM-based languages since the JVM uses signed 32-bit
integers for its size fields internally.
