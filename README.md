# `colerr`

<p align="center">
  <a href="https://travis-ci.org/dpc/colerr">
      <img src="https://img.shields.io/travis/dpc/colerr/master.svg?style=flat-square" alt="Build Status">
  </a>
  <a href="https://gitter.im/dpc/mioco">
      <img src="https://img.shields.io/badge/GITTER-join%20chat-green.svg?style=flat-square" alt="Gitter Chat">
  </a>
  <br>
  <strong><a href="//dpc.github.io/colerr/">Documentation</a></strong>
</p>

## Introduction

`colerr` will wrap a given process and colorize it's standard error output.

`colerr` is written in [rust programming language][rust] and utilizes:
[mio][mio] and [mioco][mioco] libraries. You probably don't care, but it's kind
of important so I've mentioned it here.

[mio]: https://github.com/carllerche/mio
[mioco]: https://github.com/dpc/mioco
[rust]: http://rust-lang.org

# Building

You need [rust][rust] compiler bundled with `cargo`. Then `cargo build --release` should do the job.

Resulting binary will be in `./target/release/colerr`. Just copy it to somewhere from your `$PATH`.

```
Usage:
    colorout [--] <cmd>...
```
