# Simics Rust API

This repository contains Rust bindings and utilities for Simics and the Simics API.

## Rust Simics Hello World

To run the `hello-world` sample:

```sh
cargo run -p cargo-simics-build -- -p hello-world
```

Then, in a `simics`/`simics.bat` shell:

```simics
add-module-directory target/release
load-module HelloWorld
@hw = SIM_create_object(SIM_get_class("HelloWorld"), "hw", [])
@hw.message = "Hello, World!"
@hw.iface.HelloWorldInterface.say()
@hw.iface.HelloWorldInterface2.say2()
@hw2 = SIM_create_object(SIM_get_class("HelloWorld2"), "hw2", [])
@hw2.message = "Hello, World! (Again)"
@hw2.iface.HelloWorld2Interface.say()
@hw2.iface.HelloWorld2Interface2.say2()
```

You should see:

```txt
Hello, World!
test: Hello, World!
Hello, World! (Again)
test: Hello, World! (Again)
```
