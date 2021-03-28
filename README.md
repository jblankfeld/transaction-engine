# Transaction engine

This toy transaction engine implementation uses csv, serde deserialization and serialization, rust_decimal
for fixed-precision decimal numbers.
The application reads a buffered csv file but could be easily adapted to read from another Read trait
implementation, such as std::net::TcpStream.
I used the most basic data structure (HashMap) to solve this problem in O(n) (n being rows in file) while
keeping memory as low as possible.
It is not a multi-threaded application, the choice is deliberate in this particular case as I assumed
that csv parsing and balance calculations do not require further parallelism. In other words, this application
is rate-limited by IO rather than CPU.
In a multi TCP stream environment, I would advise to use a 1 process - 1 partition stream architecture, meaning
that each client is assigned an upstream partition which guarantees the chronological order of transaction.
The rationale behind this is consistency, scalability and simplicity of code. Without this hypothesis,
it would be difficult to maintain a chronological order without a global timestamp in the input CSV.
The app logs to stderr and covers many typical parsing issues by skipping lines instead of crashing prematurely.
I apologize for the lack of tests, I wrote an integration test but ran low of time to further test
the contents of the in-memory Write trait.

# How to run
```shell
cargo run -- tests/example0.csv
```

# How to test
The test suite consists in a single end-to-end test that acts as a sanity test for the whole app.

```shell
cargo test
```

# Assumptions

In this implementation, dispute, resolve, chargeback covers both deposit and withdrawal transactions.
I implemented withdrawal dispute as specified below:

```text
deposit,1,1,1.0
withdrawal,2,2,1.0
```

## Dispute

```text
dispute,2,2,
```

The client available funds should remain the same, their held and total
funds should increase by the amount disputed.

## Resolve

```text
resolve,2,2,
```

The client available funds should remain the same, their held and total
funds should decrease by the amount disputed.

## Chargeback

```text
chargeback,2,2,
```

The client held funds should decrease by the amount disputed, their available funds should
increase by the amount disputed and their total funds should remain the same.