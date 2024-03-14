// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! A simple crate which has a class with multiple interfaces. This crate is a test case for:
//!
//! - Registering, signing, and loading a module
//! - Declaring and instantiating multiple classes
//! - Declaring and calling functions from multiple interfaces

use simics::{
    api::{ClassCreate, Interface},
    class, interface, simics_init, FromConfObject,
};

#[class(name = "HelloWorld")]
#[derive(FromConfObject, Default)]
struct HelloWorld {
    #[class(attribute(optional))]
    pub message: String,
}

#[interface(name = "HelloWorldInterface")]
impl HelloWorld {
    fn say(&self) {
        println!("{}", self.message);
    }
}

#[interface(name = "HelloWorldInterface2")]
impl HelloWorld {
    fn say2(&self) {
        println!("test: {}", self.message);
    }
}

#[class(name = "HelloWorld2")]
#[derive(FromConfObject, Default)]
struct HelloWorld2 {
    #[class(attribute(optional))]
    pub message: String,
}

#[interface(name = "HelloWorld2Interface")]
impl HelloWorld2 {
    fn say(&self) {
        println!("{}", self.message);
    }
}

#[interface(name = "HelloWorld2Interface2")]
impl HelloWorld2 {
    fn say2(&self) {
        println!("test: {}", self.message);
    }
}

#[simics_init(name = "HelloWorld", class = "HelloWorld", class = "HelloWorld2")]
fn init() {
    let hw = HelloWorld::create().expect("Failed to create class");
    HelloWorldInterface::register(hw).expect("Failed to register class interface");
    HelloWorldInterface2::register(hw).expect("Failed to register class interface");
    let hw = HelloWorld2::create().expect("Failed to create class");
    HelloWorld2Interface::register(hw).expect("Failed to register class interface");
    HelloWorld2Interface2::register(hw).expect("Failed to register class interface");
}
