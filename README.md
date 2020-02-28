# Xous Tools

This repository contains build tools for Xous, used to package up the
kernel and initial program images and create something that the runtime
can use.

It contains a number of programs:

* **copy-object**: A reimplementation of `objcopy`
* **create-image**: Tool used to create a boot args struct for Xous
* **make-tags**: Test program used to create raw boot arg tags
* **read-tags**: Test program to verify the tags were created

## Building

To build this repository, you will need Rust.

1. Build the tools: `cargo build --release`

## Using

The two most useful tools are `copy-object` and `create-image`.

To use `copy-object`, simply run `target/release/copy-object` and
specify the elf file you would like to copy.

To create a tags file with `create-image`, you will need to specify the
path to the kernel, as well as any initial programs you would like to
run.  You will also need to specify the memory range, or pass a
`csr.csv` file as an argument.

For example:

```
target/release/create-image \
      --kernel ../kernel/target/riscv32i-unknown-none-elf/debug/xous-kernel \
      --csv ../betrusted-soc/test/csr.csv \
      --init ../shell/target/riscv32i-unknown-none-elf/debug/xpr \
      args.bin
```

## Testing

_TBD_

## Contribution Guidelines

[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-v2.0%20adopted-ff69b4.svg)](CODE_OF_CONDUCT.md)

Please see [CONTRIBUTING](CONTRIBUTING.md) for details on
how to make a contribution.

Please note that this project is released with a
[Contributor Code of Conduct](CODE_OF_CONDUCT.md).
By participating in this project you agree to abide its terms.

## License

Copyright © 2020

Licensed under the [Apache License 2.0](http://opensource.org/licenses/Apache-2.0) [LICENSE](LICENSE)
