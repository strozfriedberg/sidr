#[cfg(test)]
//use wsa_lib::*;
//use super::*;
use std::env;

#[test]
fn compare_with_sql_select() {
    let key = "WSA_TEST_WINDOWS_DB";
    match env::var(key) {
        Ok(val) => println!("{key}: {val:?}"),
        Err(e) => panic!("Error getting environment variable '{key}': {e}"),
    }
}
