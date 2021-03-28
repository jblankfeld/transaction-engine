use std::process;
use transaction_engine::run;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}
