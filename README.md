# Chainlink Terra

## Developing

### Prerequisites

Before starting, make sure you have [rustup](https://rustup.rs/) along with a
recent `rustc` and `cargo` version installed (1.44.1+).

```sh
rustc --version
cargo --version
rustup target list --installed
```

And you need to have the `wasm32-unknown-unknown` target installed as well.
In case it is missing.

```sh
rustup target add wasm32-unknown-unknown
```

### Compiling and running tests

Now that you created your custom contract, make sure you can compile and run it before
making any changes. Go into the repository and do:

```sh
# this will produce a wasm build in ./target/wasm32-unknown-unknown/release/YOUR_NAME_HERE.wasm for every contract
cargo wasm

# this runs unit tests with helpful backtraces
RUST_BACKTRACE=1 cargo unit-test

# You can also run these commands only for a specific contract. E.g.
cargo wasm -p chainlink-client
```

#### Understanding the tests

The main code for each contract is in `src/contract.rs` and the unit tests there run in pure rust,
which makes them very quick to execute and give nice output on failures, especially
if you do `RUST_BACKTRACE=1 cargo unit-test`.

We consider testing critical for anything on a blockchain, and recommend to always keep
the tests up to date.

### Generating JSON Schema

While the Wasm calls (`init`, `handle`, `query`) accept JSON, this is not enough
information to use it. We need to expose the schema for the expected messages to the
clients. You can generate this schema by calling `cargo schema`, which will output
4 files in `./YOUR_CONTRACT/schema`, corresponding to the 3 message types the contract accepts,
as well as the internal `State`.


```sh
# auto-generate json schema

cd ./contracts/YOUR_CONTRACT
cargo schema
```

These files are in standard json-schema format, which should be usable by various
client side tools, either to auto-generate codecs, or just to validate incoming
json wrt. the defined schema.

### Preparing the Wasm bytecode for production

Before we upload it to a chain, we need to ensure the smallest output size possible,
as this will be included in the body of a transaction. We also want to have a
reproducible build process, so third parties can verify that the uploaded Wasm
code did indeed come from the claimed rust code.

To solve both these issues, we are using `rust-optimizer` (or to be specific in our case - `workspace-optimizer`), a docker image to
produce an extremely small build output in a consistent manner. The suggested way
to run it is this:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.11.2
```
Or using the script that does the same:
```sh
./script/optimize.sh
```

We must mount the contract code to `/code`. You can use a absolute path instead
of `$(pwd)` if you don't want to `cd` to the directory first. The other two
volumes are nice for speedup. Mounting `/code/target` in particular is useful
to avoid docker overwriting your local dev files with root permissions.
Note the `/code/target` cache is unique for each contract being compiled to limit
interference, while the registry cache is global.

This is rather slow compared to local compilations, especially the first compile
of a given contract. The use of the two volume caches is very useful to speed up
following compiles of the same contract.

This produces an `artifacts` directory in the root of the workspace. It contains the compiled and optimized WebAssembly output of the contracts along with a Sha256 checksum in `hash.txt`.

### Testing production build

Once we have this compressed `contract.wasm`, we may want to ensure it is actually
doing everything it is supposed to (as it is about 4% of the original size).
If you update the "WASM" line in `tests/integration.rs`, it will run the integration
steps on the optimized build, not just the normal build. I have never seen a different
behavior, but it is nice to verify sometimes.

```rust
static WASM: &[u8] = include_bytes!("../contract.wasm");
```

Note that this is the same (deterministic) code you will be uploading to
a blockchain to test it out, as we need to shrink the size and produce a
clear mapping from wasm hash back to the source code.

## Contributing

For commit message naming convention we use [conventional commits](https://www.conventionalcommits.org/). While this is not enforced, please try to stick to this as it eases the reviewers and also allows us to build automated `changelog` directly out of the commit messages if compliant to the format.

We use the [gitflow](https://danielkummer.github.io/git-flow-cheatsheet/) workflow [this is also helpful](https://gist.github.com/JamesMGreene/cdd0ac49f90c987e45ac).
* Development of features happens in branches made from `develop` called `feature/<the-feature>` like `feature/human-address`.
* When development is finished a pull request to `develop` is created. At least one person has to review the PR and when everything is fine the PR gets merged.
* To make a new release create a release branch called `release/X.X.X`, also bump the version number in this branch.
* Create a PR to `main` which then also has to be accepted.
* Create a tag for this version and push the tag.
* Also merge back the changes (like the version bump) into `develop`.
* The `main` branch has to be deployed to the [production environment]() automatically after PR merge.

### Rules
- Use `rebase` instead of `merge` to update your codebase, except when a PR gets included in a branch.
- Use meaningful descriptions and titles in commit messages.
- Explain what you did in your PRs, add images whenever possible for showing the status before/after the change visually.
