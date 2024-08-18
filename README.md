# Nash

A simple and modern scripting language. The concept was to create a language that is designed from the ground up to run as a script.

The difference from other shell languages like bash `bash`, is that it is not deigned to be used in an interactive way. However its different from some more traditional interpreted programming languages like `python` or `javascript` as the focus is on writing simple scripts that interact with the system directly with minimal setup required.

The idea is to create a language that has much less concepts and syntax to understand that a more fully featured programming language, but without a lot of the shell gotchas that exist in most shells today.

## Getting started

Currently there are no releases so you will need to build your own binary to use it. In order to build the command you will need to install rust, for docs on that see [here](https://www.rust-lang.org/tools/install).

Once you have rust installed cloning and building the repo can be done with the following commands:

```bash
git clone https://github.com/robert-mccausland/nash.git
cd nash
cargo build
```

## Tests

This repo contains some unit tests and example scripts.

### Unit tests

To run the unit tests run `cargo test`, see the rust documentation on running of rust unit tests.

### Example scripts

The example scripts should all run correctly, in order to have more consistency between different hosts these should be run from inside a container.

```bash
docker compose up --build -d
docker compose exec nash bash
cd /home/nash
./scripts/run-examples
```
