RUST_LOG=trace

export BAL_PUSHER_DB_FILE="$(pwd)/bal.db"
#export BAL_PUSHER_BITCOIN_COOKIE_FILE=/~/.bitcoin/.cookie
#export BAL_PUSHER_REGTEST_COOKIE_FILE=/~/.bitcoin/regtest/.cookie
#export BAL_PUSHER_TESTNET_COOKIE_FILE=/~/.bitcoin/testnet3/.cookie
#export BAL_PUSHER_SIGNET_COOKIE_FILE=/~/.bitcoin/signet/.cookie

export BAL_PUSHER_REGTEST_ZMQ_HASHBLOCK=tcp://127.0.0.1:21332
export BAL_PUSHER_SEND_STATS=true 
export WELIST_SERVER_URL=http://localhost:8085
export BAL_SERVER_URL="http://127.0.0.1:9133"
export SSL_KEY_PATH="$(pwd)/private_key.pem"
cargo run --bin=bal-pusher regtest
