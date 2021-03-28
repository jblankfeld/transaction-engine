use csv::{ReaderBuilder, Trim};
use log::error;
use rust_decimal::Decimal;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufReader, Write};
use std::ops::{Add, Sub};
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct Input {
    #[serde(rename = "type")]
    pub operation: Operation,
    #[serde(rename = "client", deserialize_with = "from_str")]
    pub client_id: u16,
    #[serde(rename = "tx", deserialize_with = "from_str")]
    pub transaction_id: u32,
    #[serde(deserialize_with = "from_str_optional")]
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub transaction_id: u32,
    pub operation: Operation,
    pub amount: Option<Decimal>,
    pub is_disputed: bool,
}

impl Transaction {
    pub fn from_input(input: Input) -> Transaction {
        Transaction {
            transaction_id: input.transaction_id,
            operation: input.operation,
            amount: input.amount,
            is_disputed: false,
        }
    }

    pub fn set_dispute(&mut self, is_disputed: bool) {
        self.is_disputed = is_disputed;
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

// CSV output model
#[derive(Debug, Serialize)]
pub struct AccountStatus {
    #[serde(rename = "client")]
    client_id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

pub struct Client {
    pub account_status: AccountStatus,
    pub transactions: HashMap<u32, Transaction>,
}

impl Client {
    pub fn new(client_id: u16) -> Client {
        Client {
            account_status: AccountStatus::new(client_id),
            transactions: HashMap::new(),
        }
    }

    pub fn into_account_status(self) -> AccountStatus {
        self.account_status
    }

    pub fn deposit(&mut self, transaction: Transaction) {
        match transaction.amount {
            Some(amount) => {
                self.account_status.available = self.account_status.available.add(amount);
                self.account_status.total = self.account_status.total.add(amount);

                self.transactions
                    .insert(transaction.transaction_id, transaction);
            }
            None => error!("invalid deposit - no amount for tx {:?}", transaction),
        }
    }

    pub fn withdrawal(&mut self, transaction: Transaction) {
        match transaction.amount {
            Some(amount) => {
                if self.account_status.available > amount {
                    self.account_status.available = self.account_status.available.sub(amount);
                    self.account_status.total = self.account_status.total.sub(amount);

                    self.transactions
                        .insert(transaction.transaction_id, transaction);
                } else {
                    error!(
                        "invalid withdrawal - not enough funds for tx {:?}",
                        transaction
                    );
                }
            }
            None => error!("invalid withdrawal - no amount for tx {:?}", transaction),
        }
    }

    pub fn dispute(&mut self, transaction: Transaction) {
        match self.transactions.get_mut(&transaction.transaction_id) {
            Some(disputed) => match disputed.amount {
                Some(amount) => {
                    disputed.set_dispute(true);
                    match disputed.operation {
                        Operation::Deposit => {
                            self.account_status.available =
                                self.account_status.available.sub(amount);
                            self.account_status.held = self.account_status.held.add(amount);
                        }
                        Operation::Withdrawal => {
                            self.account_status.held = self.account_status.held.add(amount);
                            self.account_status.total = self.account_status.total.add(amount);
                        }
                        _ => error!("invalid dispute - bad operation - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                    }
                }
                None => error!(
                    "invalid dispute - no amount - for tx: {:?} and alleged dispute tx: {:?}",
                    transaction, disputed
                ),
            },
            None => error!(
                "invalid dispute - disputed tx not found - for tx: {:?}",
                transaction
            ),
        }
    }

    pub fn resolve(&mut self, transaction: Transaction) {
        match self.transactions.get_mut(&transaction.transaction_id) {
            Some(disputed) => {
                if disputed.is_disputed {
                    match disputed.amount {
                        Some(amount) => match disputed.operation {
                            Operation::Deposit => {
                                self.account_status.available =
                                    self.account_status.available.add(amount);
                                self.account_status.held = self.account_status.held.sub(amount);
                            }
                            Operation::Withdrawal => {
                                self.account_status.held = self.account_status.held.sub(amount);
                                self.account_status.total = self.account_status.total.sub(amount);
                            }
                            _ => error!("invalid resolve - bad operation - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                        },
                        None => error!("invalid resolve - no amount - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                    }
                } else {
                    error!("invalid resolve - alleged dispute not disputed - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                }
            }
            None => error!(
                "invalid resolve - disputed tx not found - for tx: {:?}",
                transaction
            ),
        }
    }

    pub fn chargeback(&mut self, transaction: Transaction) {
        match self.transactions.get_mut(&transaction.transaction_id) {
            Some(disputed) => {
                if disputed.is_disputed {
                    match disputed.amount {
                        Some(amount) => {
                            self.account_status.locked = true;
                            match disputed.operation {
                                Operation::Deposit => {
                                    self.account_status.held = self.account_status.held.sub(amount);
                                    self.account_status.total = self.account_status.total.sub(amount);
                                }
                                Operation::Withdrawal => {
                                    self.account_status.available =
                                        self.account_status.available.add(amount);
                                    self.account_status.held = self.account_status.held.sub(amount);
                                }
                                _ => error!("invalid chargeback - bad operation - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                            }
                        },
                        None => error!("invalid chargeback - no amount - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                    }
                } else {
                    error!("invalid chargeback - alleged dispute not disputed - for tx: {:?} and alleged dispute tx: {:?}", transaction, disputed)
                }
            }
            None => error!(
                "invalid chargeback - disputed tx not found - for tx: {:?}",
                transaction
            ),
        }
    }
}

impl AccountStatus {
    pub fn new(client_id: u16) -> AccountStatus {
        AccountStatus {
            client_id,
            available: Decimal::new(0, 0),
            held: Decimal::new(0, 0),
            total: Decimal::new(0, 0),
            locked: false,
        }
    }

    pub fn round_and_normalize(&mut self) {
        self.available = self.available.round_dp(4).normalize();
        self.held = self.held.round_dp(4).normalize();
        self.total = self.total.round_dp(4).normalize();
    }
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    T::from_str(String::deserialize(deserializer)?.trim()).map_err(de::Error::custom)
}

fn from_str_optional<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)
        .map(|s| {
            T::from_str(&s[..])
                .map(|dec| Some(dec))
                .map_err(|err| {
                    error!("Error while parsing string: {}", err);
                    err
                })
                .unwrap_or(None) // Skip errors
        })
        .map_err(|err| {
            error!("Error while deserializing field: {:?}", err);
            err
        })
        .or(Ok(None)) // Skip errors
}

fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("1st argument must be a file path")),
        Some(file_path) => Ok(file_path),
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let file_path = get_first_arg()?;
    process_file(file_path, Box::new(std::io::stdout()))
}

pub fn process_file(file_path: OsString, out: Box<dyn Write>) -> Result<(), Box<dyn Error>> {
    let file = File::open(file_path)?;

    let mut rdr = ReaderBuilder::new()
        .flexible(true)
        .trim(Trim::All)
        .from_reader(BufReader::new(file));

    let mut clients: HashMap<u16, Client> = HashMap::new();

    for result in rdr.deserialize() {
        match result {
            Ok(record) => {
                let record: Input = record;

                let client = clients
                    .entry(record.client_id)
                    .or_insert(Client::new(record.client_id));

                let transaction = Transaction::from_input(record);

                match transaction.operation {
                    Operation::Deposit => {
                        client.deposit(transaction);
                    }
                    Operation::Withdrawal => {
                        client.withdrawal(transaction);
                    }
                    Operation::Dispute => {
                        client.dispute(transaction);
                    }
                    Operation::Resolve => {
                        client.resolve(transaction);
                    }
                    Operation::Chargeback => {
                        client.chargeback(transaction);
                    }
                }
            }
            Err(err) => {
                error!("parsing error: {}", err);
            }
        }
    }

    let mut wtr = csv::Writer::from_writer(out);

    for (_, mut client) in clients.drain() {
        client.account_status.round_and_normalize();
        wtr.serialize(client.into_account_status())?;
    }

    Ok(())
}
