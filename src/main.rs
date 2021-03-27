use std::env;
use std::fmt::Display;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;
use std::str::FromStr;
use rust_decimal::prelude::*;

use serde::{de, Deserialize, Deserializer};
use std::collections::HashMap;
use std::ops::{Add, Sub};

#[derive(Debug, Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    operation: Operation,
    #[serde(rename = "client", deserialize_with = "from_str")]
    client_id: u16,
    #[serde(rename = "tx", deserialize_with = "from_str")]
    transaction_id: u32,
    #[serde(deserialize_with = "from_str_optional")]
    amount: Option<Decimal>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Operation {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback
}

#[derive(Debug, Deserialize)]
struct Balance {
    client_id: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where T: FromStr,
          T::Err: Display,
          D: Deserializer<'de>
{
    T::from_str(String::deserialize(deserializer)?.trim()).map_err(de::Error::custom)
}

fn from_str_optional<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where T: FromStr,
          T::Err: Display,
          D: serde::Deserializer<'de>
{
    let deser_res = String::deserialize(deserializer);
    match deser_res {
        Ok(v) => {
            let parsed = T::from_str(v.trim()).map_err(de::Error::custom)?;
            Ok(Some(parsed))
        },
        Err(_) => Ok(None)
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut balances: HashMap<u16, Balance> = HashMap::new();

    for result in rdr.deserialize() {
        let transaction: Transaction = result?;

        let client_balance = balances
            .entry(transaction.client_id)
            .or_insert(Balance {
                client_id: transaction.client_id,
                available: Decimal::zero(),
                held: Decimal::zero(),
                total: Decimal::zero(),
                locked: false
            });

        let r = transaction.amount.unwrap_or_default();

        match transaction.operation {
            Operation::Deposit => {
                client_balance.available = client_balance.available.add(r)
            }
            Operation::Withdrawal => {
                client_balance.available = client_balance.available.sub(r)
            }
            Operation::Dispute => { }
            Operation::Resolve => { }
            Operation::Chargeback => { }
        }

        println!("{:?}", transaction);
        println!("{:?}", client_balance);
    }

    Ok(())
}

fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}