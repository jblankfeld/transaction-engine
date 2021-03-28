extern crate transaction_engine;

mod tests {
    use std::ffi::OsStr;
    use std::io::Cursor;
    use transaction_engine::*;

    // Simple integration test that writes to an in-memory Write trait (Cursor)
    #[test]
    fn sanity_test() {
        let os_str = OsStr::new("tests/example0.csv");
        let cursor = Box::new(Cursor::new(Vec::new()));

        if let Err(err) = process_file(os_str.to_os_string(), cursor) {
            println!("{}", err);
            assert!(false);
        }

        // TODO: test the content of Cursor against the expected results
    }
}
