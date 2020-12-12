Purpose
=======

This tool processes a list of things that you might want to buy. It consolidates
the list by adding up stuff that has multiple appearances on the list. If you
provide a database in yaml format, it will also map the items to merchants.

Prerequisites
=============

1. Install Rust as described at https://rustup.rs/ and add its binaries to
   PATH.
2. Install Node.js.
3. Install wasm-pack by issuing `cargo install wasm-pack`.

Build
=====

Command line
------------

1. Go to magiczettel/magiczettel.
2. Call `cargo build --release`.

Web
---

1. Go to magiczettel/shoppinglist.
2. Call `wasm-pack build --release`.
3. Go to magiczettel/shoppinglist/www.
4. Call `npm install`.
5. Call `npm run build`.