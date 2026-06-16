# bal-server

## Installation
```bash
$ git clone ....
$ cd bal-server
$ openssl genpkey -algorithm ED25519 -out private_key.pem
$ openssl pkey -in private_key.pem -pubout -out public_key.pem
$ cargo build --release
$ sudo cp target/release/bal-server /usr/local/bin
$ bal-server
```

## Configuration

The `bal-server` application can be configured using environment variables. The following variables are available:

| Variable | Description | Default |
| --- | --- | --- |
| `BAL_SERVER_CONFIG_FILE` | Path to the configuration file. If the file does not exist, a new one will be created. | `$HOME/.config/bal-server/default-config.toml` |
| `BAL_SERVER_DB_FILE` | Path to the SQLite3 database file. If the file does not exist, a new one will be created. | `bal.db` |
| `BAL_SERVER_BIND_ADDRESS` | Public address for listening to requests. | `127.0.0.1` |
| `BAL_SERVER_BIND_PORT` | Default port for listening to requests. | `9137` |
| `BAL_SERVER_PUB_KEY_PATH` | WillExecutor Ed25519 public key | `public_key.pem` |
| `BAL_SERVER_REGTEST_ADDRESS` | Bitcoin address for the regtest environment. | - |
| `BAL_SERVER_REGTEST_FIXED_FEE` | Fixed fee for the regtest environment. | 50000 |
| `BAL_SERVER_SIGNET_ADDRESS` | Bitcoin address for the signet environment. | - |
| `BAL_SERVER_SIGNET_FIXED_FEE` | Fixed fee for the signet environment. | 50000 |
| `BAL_SERVER_TESTNET_ADDRESS` | Bitcoin address for the testnet environment. | - |
| `BAL_SERVER_TESTNET_FIXED_FEE` | Fixed fee for the testnet environment. | 50000 |
| `BAL_SERVER_BITCOIN_ADDRESS` | Bitcoin address for the mainnet environment. | - |
| `BAL_SERVER_BITCOIN_FIXED_FEE` | Fixed fee for the mainnet environment. | 50000 |


# bal-pusher

`bal-pusher` is a tool that retrieves Bitcoin transactions from a database and pushes them to the Bitcoin network when their **locktime** exceeds the **median time past** (MTP). It listens for Bitcoin block updates via ZMQ.

## Installation

To use `bal-pusher`, you need to compile and install Bitcoin with ZMQ (ZeroMQ) support enabled. Then, configure your Bitcoin node and `bal-pusher` to push the transactions.

### Prerequisites

1. **Bitcoin with ZMQ Support**:
   Ensure that Bitcoin is compiled with ZMQ support. Add the following line to your `bitcoin.conf` file:

   ```
   zmqpubhashblock=tcp://127.0.0.1:28332
   ```

2. **Install Rust and Cargo**:
   If you haven't already installed Rust and Cargo, you can follow the official instructions to do so: [Rust Installation](https://www.rust-lang.org/tools/install).

## Configuration

`bal-pusher` can be configured using environment variables. If no configuration file is provided, a default configuration file will be created.

### Available Configuration Variables

| Variable                              | Description                              | Default                                      |
|---------------------------------------|------------------------------------------|----------------------------------------------|
| `BAL_PUSHER_CONFIG_FILE`              | Path to the configuration file. If the file does not exist, it will be created. | `$HOME/.config/bal-pusher/default-config.toml` |
| `BAL_PUSHER_DB_FILE`                  | Path to the SQLite3 database file. If the file does not exist, it will be created. | `bal.db`                                      |
| `BAL_PUSHER_ZMQ_LISTENER`             | ZMQ listener for Bitcoin updates.        | `tcp://127.0.0.1:28332`                      |
| `BAL_PUSHER_BITCOIN_HOST`             | Bitcoin server host for RPC connections. | `http://127.0.0.1`                           |
| `BAL_PUSHER_BITCOIN_PORT`             | Bitcoin RPC server port.                 | `8332`                                       |
| `BAL_PUSHER_BITCOIN_COOKIE_FILE`      | Path to Bitcoin RPC cookie file.         | `$HOME/.bitcoin/.cookie`                     |
| `BAL_PUSHER_BITCOIN_RPC_USER`         | Bitcoin RPC username.                    | -                                            |
| `BAL_PUSHER_BITCOIN_RPC_PASSWORD`     | Bitcoin RPC password.                    | -                                            |
| `BAL_PUSHER_SEND_STATS`               | Contact welist to provide times          | false                                        |
| `WELIST_SERVER_URL`                   | welist server url to provide times       | https://welist.bitcoin-afer.life             |
| `BAL_SERVER_URL`                      | WillExecutor server url                  | -                                            |
| `SSL_KEY_PATH`                        | Ed25519 private key pem file             | `private_key.pem`                            |


## Running `bal-pusher`

Once the application is installed and configured, you can start `bal-pusher` by running the following command:

```bash
$ bal-pusher [bitcoin|testnet|regtest|]
```

This will start the service, which will listen for Bitcoin blocks via ZMQ and push transactions from the database when their locktime exceeds the median time past.
