//! Every way Rust lets you spell the same call. Syntax tells them apart;
//! resolution does not.
use std::fs;
use std::fs::read_to_string;

pub fn qualified() {
    let _ = std::fs::read("a");
}

pub fn via_module() {
    let _ = fs::read("b");
}

pub fn via_item() {
    let _ = read_to_string("c");
}

pub fn local_caller() {
    via_module();
}
