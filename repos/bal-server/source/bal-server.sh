WORKING_DIR=$(pwd)
if [ ! -f "$WORKING_DIR/public_key.pem" ]; then
  echo "creating keypairs"
  openssl genpkey -algorithm ED25519 -out private_key.pem
  openssl pkey -in private_key.pem -pubout -out public_key.pem
fi

export RUST_LOG="trace"
export BAL_SERVER_DB_FILE="$WORKING_DIR/bal.db"
export BAL_SERVER_INFO="BAL devel willexecutor server"
export BAL_SERVER_BIND_ADDRESS="127.0.0.1"
export BAL_SERVER_BIND_PORT=9133
export BAL_SERVER_PUB_KEY_PATH="$WORKING_DIR/public_key.pem"
export BAL_SERVER_EXPOSE_STATS=true;

#export BAL_SERVER_BITCOIN_ADDRESS="your bitcoin address or xpub to recive payments here"
#export BAL_SERVER_BITCOIN_FIXED_FEE=50000

export BAL_SERVER_REGTEST_ADDRESS="vpub5UhLrYG1qQjnJhvJgBdqgpznyH11mxW9hwBYxf3KhfdjiupCFPUVDvgwpeZ9Wj5YUJXjKjXjy7DSbJNBW1sXbKwARiaphm1UjHYy3mKvTG4"
export BAL_SERVER_REGTEST_FIXED_FEE=1000
#export BAL_SERVER_TESTNET_ADDRESS=
#export BAL_SERVER_TESTNET_FEE=100000
#export BAL_SERVER_SIGNET_ADDRESS=
#export BAL_SERVER_SIGNET_FEE=100000
cargo run --bin=bal-server
