macro_rules! assert_ok {
    ($s: expr) => {
        match $s {
            Ok(x) => x,
            Err(e) => panic!(format!("Value not OK: {:?}", e))
        }
    }
}