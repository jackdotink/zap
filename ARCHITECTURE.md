# Architecture

Zap operates in three main phases: Api, Hir, and Mir.

## Api

Api is simple. It is what the user returns to zap. The Api phase includes the zap runtime itself and its results.

Relevant file: [api.rs](./src/api.rs)

## Hir

Hir (high-level intermediate representation) takes the somewhat unwieldy types from Api and transforms them into something easier to work with. In the future, some optimizations will be applied in this transformation, for example turning some types into specific optimized types, like an array of booleans into a `BooleanArray` type.

Relevant file: [hir.rs](./src/hir/mod.rs)

## Mir

Mir (mid-level intermediate representation) takes the Hir and transforms it into instructions that map fairly closely to actual Luau. Hir itself is split into a few parts.

In the future Mir will run some basic optimizations.

Relevant file: [mir.rs](./src/mir/mod.rs)

### Serdes

The serdes module actually transforms Hir *types* into instructions to serialize and deserialize those values. Serdes is effected by native codegen being enabled, and by other options. Serdes is also where checks are emitted.

Serdes operates using a system of callbacks. Any static variables are emitted outside the relevant function, and then later, those static variables can be used within the function.

Relevant file: [serdes.rs](./src/mir/serdes.rs)

### Client and Server

The client and server modules output the respective api for each side of the network. They are each split into several files to handle different aspects of the api, such as a file for an iterating api and a file for a sending api.

Relevant files: [client.rs](./src/mir/client/mod.rs), [server.rs](./src/mir/server/mod.rs)