//! One method, several receivers. Only resolution tells them apart.
use std::sync::Arc;

pub fn clone_an_arc(handle: &Arc<String>) -> Arc<String> {
    handle.clone()
}

pub fn clone_a_string(value: &String) -> String {
    value.clone()
}

pub fn clone_a_vec(value: &Vec<u8>) -> Vec<u8> {
    value.clone()
}

pub fn collected(values: &[u8]) -> Vec<u8> {
    values.iter().copied().collect()
}
