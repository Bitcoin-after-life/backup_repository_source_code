extern crate bitcoincore_rpc;
extern crate zmq;
use bitcoin::Network;

use bitcoincore_rpc::{Auth, Client, Error, RpcApi, bitcoin};
use bitcoincore_rpc_json::GetBlockchainInfoResult;

use byteorder::{LittleEndian, ReadBytesExt};
use hex;
use log::{debug, error, info, trace, warn};
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use sqlite::{Connection, Value};
use std::collections::HashMap;
use std::env;
use std::error::Error as StdError;
use std::io::Cursor;
use std::str;
use std::{thread, time::Duration};
use zmq::{Context, DEALER, DONTWAIT, Socket};

use base64::{Engine as _, engine::general_purpose};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use openssl::sign::Verifier;
use reqwest::Client as rClient;
use std::fs;
use std::time::Instant;

const LOCKTIME_THRESHOLD: i64 = 5000000;
const VERSION: &str = "0.0.2";
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyConfig {
    zmq_listener: String,
    db_file: String,
    bitcoin_dir: String,
    regtest: NetworkParams,
    testnet: NetworkParams,
    testnet4: NetworkParams,
    signet: NetworkParams,
    mainnet: NetworkParams,
    send_stats: bool,
    url: String,
    ssl_key_path: String,
}

impl Default for MyConfig {
    fn default() -> Self {
        MyConfig {
            zmq_listener: env::var("BAL_PUSHER_ZMQ_LISTENER")
                .unwrap_or("tcp://127.0.0.1:28332".to_string()),
            db_file: env::var("BAL_PUSHER_DB_FILE").unwrap_or("bal.db".to_string()),
            bitcoin_dir: env::var("BAL_PUSHER_BITCOIN_DIR").unwrap_or("".to_string()),
            regtest: get_network_params_default(Network::Regtest),
            testnet: get_network_params_default(Network::Testnet),
            testnet4: get_network_params_default(Network::Testnet4),
            signet: get_network_params_default(Network::Signet),
            mainnet: get_network_params_default(Network::Bitcoin),
            send_stats: env::var("BAL_PUSHER_SEND_STATS")
                .unwrap_or("false".to_string())
                .parse::<bool>()
                .unwrap(),
            url: env::var("BAL_SERVER_URL").unwrap_or("http://localhost/".to_string()),
            ssl_key_path: env::var("SSL_KEY_PATH").unwrap_or("privkey.pem".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkParams {
    host: String,
    port: u16,
    dir_path: String,
    db_field: String,
    cookie_file: String,
    rpc_user: String,
    rpc_pass: String,
    zmq_listener:String
}
fn get_network_params(cfg: &MyConfig, network: Network) -> &NetworkParams {
    match network {
        Network::Testnet => &cfg.testnet,
        Network::Testnet4 => &cfg.testnet4,
        Network::Signet => &cfg.signet,
        Network::Regtest => &cfg.regtest,
        _ => &cfg.mainnet,
    }
}
fn get_network_params_default(network: Network) -> NetworkParams {
    match network {
        Network::Testnet => NetworkParams {
            host: "http://i27.0.0.1".to_string(),
            port: 18332,
            dir_path: "testnet3/".to_string(),
            db_field: "testnet".to_string(),
            cookie_file: "".to_string(),
            rpc_user: "".to_string(),
            rpc_pass: "".to_string(),
            zmq_listener: "tcp://127.0.0.1:23332".to_string(),
        },
        Network::Testnet4 => NetworkParams {
            host: "http://i27.0.0.1".to_string(),
            port: 18332,
            dir_path: "testnet4/".to_string(),
            db_field: "testnet4".to_string(),
            cookie_file: "".to_string(),
            rpc_user: "".to_string(),
            rpc_pass: "".to_string(),
            zmq_listener: "tcp://127.0.0.1:24332".to_string(),
        },
        Network::Signet => NetworkParams {
            host: "http://127.0.0.1".to_string(),
            port: 18332,
            dir_path: "signet/".to_string(),
            db_field: "signet".to_string(),
            cookie_file: "".to_string(),
            rpc_user: "".to_string(),
            rpc_pass: "".to_string(),
            zmq_listener: "tcp://127.0.0.1:22332".to_string(),
        },
        Network::Regtest => NetworkParams {
            host: "http://127.0.0.1".to_string(),
            port: 18443,
            dir_path: "regtest/".to_string(),
            db_field: "regtest".to_string(),
            cookie_file: "".to_string(),
            rpc_user: "".to_string(),
            rpc_pass: "".to_string(),
            zmq_listener: "tcp://127.0.0.1:21332".to_string(),
        },
        _ => NetworkParams {
            host: "http://127.0.0.1".to_string(),
            port: 8332,
            dir_path: "".to_string(),
            db_field: "bitcoin".to_string(),
            cookie_file: "".to_string(),
            rpc_user: "".to_string(),
            rpc_pass: "".to_string(),
            zmq_listener: "tcp://127.0.0.1:28332".to_string(),
        },
    }
}

fn get_cookie_filename(network: &NetworkParams) -> Result<String, Box<dyn StdError>> {
    if network.cookie_file != "" {
        Ok(network.cookie_file.clone())
    } else {
        match env::var_os("HOME") {
            Some(home) => match home.to_str() {
                Some(home_str) => {
                    let cookie_file_path =
                        format!("{}/.bitcoin/{}.cookie", home_str, network.dir_path);

                    Ok(cookie_file_path)
                }
                None => Err("wrong HOME value".into()),
            },
            None => Err("Please Set HOME environment variable".into()),
        }
    }
}
fn get_client_from_username(
    url: &String,
    network: &NetworkParams,
) -> Result<(Client, GetBlockchainInfoResult), Box<dyn StdError>> {
    if network.rpc_user != "" {
        match Client::new(
            &url[..],
            Auth::UserPass(network.rpc_user.to_string(), network.rpc_pass.to_string()),
        ) {
            Ok(client) => match client.get_blockchain_info() {
                Ok(bcinfo) => Ok((client, bcinfo)),
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err.into()),
        }
    } else {
        Err("Failed".into())
    }
}
fn get_client_from_cookie(
    url: &String,
    network: &NetworkParams,
) -> Result<(Client, GetBlockchainInfoResult), Box<dyn StdError>> {
    match get_cookie_filename(network) {
        Ok(cookie) => match Client::new(&url[..], Auth::CookieFile(cookie.into())) {
            Ok(client) => match client.get_blockchain_info() {
                Ok(bcinfo) => Ok((client, bcinfo)),
                Err(err) => Err(err.into()),
            },
            Err(err) => Err(err.into()),
        },
        Err(err) => Err(err.into()),
    }
}
fn get_client(
    network: &NetworkParams,
) -> Result<(Client, GetBlockchainInfoResult), Box<dyn StdError>> {
    let url = format!("{}:{}/", network.host, &network.port);
    match get_client_from_username(&url, network) {
        Ok(client) => Ok(client),
        Err(_) => match get_client_from_cookie(&url, &network) {
            Ok(client) => Ok(client),
            Err(err) => Err(err.into()),
        },
    }
}
async fn main_result(cfg: &MyConfig, network_params: &NetworkParams) -> Result<(), Error> {
    /*let url = args.next().expect("Usage: <rpc_url> <username> <password>");
    let user = args.next().expect("no user given");
    let pass = args.next().expect("no pass given");
    */
    //let network = Network::Regtest
    match get_client(network_params) {
        Ok((rpc, bcinfo)) => {
            info!("connected");
            //let best_block_hash = rpc.get_best_block_hash()?;
            //info!("best block hash: {}", best_block_hash);
            //let bestblockcount = rpc.get_block_count()?;
            //info!("best block height: {}", bestblockcount);
            //let best_block_hash_by_height = rpc.get_block_hash(bestblockcount)?;
            //info!("best block hash by height: {}", best_block_hash_by_height);
            //assert_eq!(best_block_hash_by_height, best_block_hash);
            //let from_block= std::cmp::max(0, bestblockcount - 11);
            //let mut time_sum:u64=0;
            //for i in from_block..bestblockcount{
            //    let hash = rpc.get_block_hash(i).unwrap();
            //    let block: bitcoin::Block = rpc.get_by_id(&hash).unwrap();
            //    time_sum += <u32 as Into<u64>>::into(block.header.time);
            //}
            //let average_time = time_sum/11;
            info!("median time: {}", bcinfo.median_time);
            //info!("height time: {}",bcinfo.median_time);
            info!("blocks: {}", bcinfo.blocks);
            debug!("best block hash: {}", bcinfo.best_block_hash);

            let average_time = bcinfo.median_time;
            let db = sqlite::open(&cfg.db_file).unwrap();
            info!("db open {}", &cfg.db_file);

            let sqlquery = "SELECT  * FROM tbl_tx WHERE network = :network AND status = :status AND ( locktime < :bestblock_height  OR locktime > :locktime_threshold AND locktime < :bestblock_time);";
            let query_tx = db.prepare(sqlquery).unwrap().into_iter();
            trace!("query_tx: {}", sqlquery);
            trace!(":locktime_threshold: {}", LOCKTIME_THRESHOLD);
            trace!(":bestblock_time: {}", average_time);
            trace!(":bestblock_height: {}", bcinfo.blocks);
            trace!(":network: {}", network_params.db_field.clone());
            trace!(":status: {}", 0);
            //let query_tx = db.prepare("SELECT * FROM tbl_tx where status = :status").unwrap().into_iter();
            let mut pushed_txs: Vec<String> = Vec::new();
            let mut invalid_txs: std::collections::HashMap<String, String> = HashMap::new();
            for row in query_tx
                .bind::<&[(_, Value)]>(
                    &[
                        (":locktime_threshold", (LOCKTIME_THRESHOLD as i64).into()),
                        (":bestblock_time", (average_time as i64).into()),
                        (":bestblock_height", (bcinfo.blocks as i64).into()),
                        (":network", network_params.db_field.clone().into()),
                        (":status", 0.into()),
                    ][..],
                )
                .unwrap()
                .map(|row| row.unwrap())
            {
                let tx = row.read::<&str, _>("tx");
                let txid = row.read::<&str, _>("txid");
                let locktime = row.read::<i64, _>("locktime");
                info!("to be pushed: {}: {}", txid, locktime);
                match rpc.send_raw_transaction(tx) {
                    Ok(o) => {
                        /*let mut file = OpenOptions::new()
                            .append(true) // Set the append option
                            .create(true) // Create the file if it doesn't exist
                            .open("valid_txs")?;
                        let data = format!("{}\t:\t{}\t:\t{}\n",txid,average_time,locktime);
                        file.write_all(data.as_bytes())?;
                        drop(file);
                        */
                        info!("tx: {} pusshata PUSHED\n{}", txid, o);
                        pushed_txs.push(txid.to_string());
                    }
                    Err(err) => {
                        /*let mut file = OpenOptions::new()
                            .append(true) // Set the append option
                            .create(true) // Create the file if it doesn't exist
                            .open("/home/bal/invalid_txs")?;
                        let data = format!("{}:\t{}\t:\t{}\t:\t{}\n",txid,err,average_time,locktime);
                        file.write_all(data.as_bytes())?;
                        drop(file);
                        */
                        warn!("Error: {}\n{}", err, txid);
                        //store err in invalid_txs
                        invalid_txs.insert(txid.to_string(), err.to_string());
                    }
                };
            }

            if pushed_txs.len() > 0 {
                let sql = format!(
                    "UPDATE tbl_tx SET status = 1 WHERE txid in ('{}');",
                    pushed_txs.join("','")
                );
                trace!("sqlok: {}", &sql);
                let _ = db.execute(&sql);
            }
            if invalid_txs.len() > 0 {
                for (txid, txerr) in &invalid_txs {
                    //let _ = db.execute(format!("UPDATE tbl_tx SET status = 2 WHERE txid in ('{}'Yp);",invalid_txs.join("','")));
                    let sql = format!(
                        "UPDATE tbl_tx SET status = 2, push_err='{txerr}' WHERE txid = '{txid}'"
                    );
                    trace!("sqlerror: {}", &sql);
                    let _ = db.execute(&sql);
                }
            }
            let _ = send_stats_report(cfg, bcinfo).await;
            let _ = calculate_stats(&db, network_params.db_field.clone()).await;
        }
        Err(erx) => {
            panic!("impossible to get client {}", erx)
        }
    }
    Ok(())
}
async fn calculate_stats(db: &Connection, chain: String) -> Result<(), reqwest::Error> {
    //let sql = "drop table if exists tbl_stats;";
    let sql = "DELETE FROM tbl_stats WHERE chain = '{chain}';";
    if let Err(err) = db.execute(&sql) {
        error!("error deleting from tbl_stats where chain:{chain} error: {err}");
    }
    let sql = format!(
        "INSERT INTO tbl_stats (
  report_date, chain, totals, waiting, sent, failed,
  waiting_profit, sent_profit, missed_profit, unique_inputs
)
VALUES (
  CURRENT_TIMESTAMP,
  '{chain}',
  (SELECT COUNT(*) FROM tbl_tx WHERE network = '{chain}'),
  (SELECT COUNT(*) FROM tbl_tx WHERE status = 0 AND network = '{chain}'),
  (SELECT COUNT(*) FROM tbl_tx WHERE status = 1 AND network = '{chain}'),
  (SELECT COUNT(*) FROM tbl_tx WHERE status = 2 AND network = '{chain}'),
  (SELECT IFNULL(SUM(our_fees),0) FROM tbl_tx WHERE status = 0 AND network = '{chain}'),
  (SELECT IFNULL(SUM(our_fees),0) FROM tbl_tx WHERE status = 1 AND network = '{chain}'),
  (SELECT IFNULL(SUM(our_fees),0) FROM tbl_tx WHERE status = 2 AND network = '{chain}'),
  (SELECT COUNT(DISTINCT tbl_inp.in_txid)
     FROM tbl_inp
     JOIN tbl_tx ON tbl_inp.txid = tbl_tx.txid
     WHERE tbl_tx.status = 0 AND tbl_tx.network = '{chain}')
)
ON CONFLICT(chain) DO UPDATE SET
  report_date = excluded.report_date,
  totals = excluded.totals,
  waiting = excluded.waiting,
  sent = excluded.sent,
  failed = excluded.failed,
  waiting_profit = excluded.waiting_profit,
  sent_profit = excluded.sent_profit,
  missed_profit = excluded.missed_profit,
  unique_inputs = excluded.unique_inputs;
  "
    );

    /*
    let sql = format!("CREATE TABLE tbl_stats AS
	SELECT
		CURRENT_TIMESTAMP AS report_date,
        '{chain}' as chain,
		(SELECT COUNT(*) FROM tbl_tx WHERE network ='{chain}') AS totals,
		(SELECT COUNT(*) FROM tbl_tx WHERE status = 0 AND network ='{chain}') AS waiting,
		(SELECT COUNT(*) FROM tbl_tx WHERE status = 1 AND network ='{chain}') AS sent,
		(SELECT COUNT(*) FROM tbl_tx WHERE status = 2 AND network ='{chain}') AS failed,
		(SELECT SUM(our_fees) FROM tbl_tx WHERE status = 0 AND network ='{chain}') AS waiting_profit,
		(SELECT SUM(our_fees) OR 0 FROM tbl_tx WHERE status = 1 AND network ='{chain}') AS sent_profit,
		(SELECT SUM(our_fees) FROM tbl_tx WHERE status = 2 AND network ='{chain}') AS missed_profit,
		(SELECT COUNT(*) FROM tbl_inp JOIN tbl_tx ON(tbl_inp.txid = tbl_tx.txid) WHERE tbl_tx.status=0 AND tbl_tx.network ='{chain}') AS unique_inputs;
    ");
    let sql = "UPDATE tbl_stats set
        totals = (SELECT COUNT(*) FROM tbl_tx WHERE network ='{chain}'),
        waiting = (SELECT COUNT(*) FROM tbl_tx WHERE status = 0 AND network ='{chain}'),
        sent = (SELECT COUNT(*) FROM tbl_tx WHERE status = 1 AND network ='{chain}'),
        failed = (SELECT COUNT(*) FROM tbl_tx WHERE status = 1 AND network ='{chain}'),
        waiting_profit = (SELECT SUM(our_fees) FROM tbl_tx WHERE status = 0 AND network ='{chain}'),
        sent_profit = (SELECT SUM(our_fees) FROM tbl_tx WHERE status = 0 AND network ='{chain}'),
        missed_profit = (SELECT SUM(our_fees) FROM tbl_tx WHERE status = 0 AND network ='{chain}')
        unique_inputs = (SELECT COUNT(*) FROM tbl_inp JOIN tbl_tx ON(tbl_inp.txid = tbl_tx.txid) WHERE tbl_tx.status=0 AND tbl_tx.network ='{chain}')
        WHERE chain = '{chain}'
    */
    if let Err(err) = db.execute(&sql) {
        error!("error inserting creating stats table {err}");
    } else {
        info!("tbl_stats creation success");
    }
    Ok(())
}
async fn send_stats_report(
    cfg: &MyConfig,
    bcinfo: GetBlockchainInfoResult,
) -> Result<(), reqwest::Error> {
    if cfg.send_stats {
        debug!("sending report to welist");
        let welist_url = env::var("WELIST_SERVER_URL")
            .unwrap_or("https://welist.bitcoin-after.life".to_string());

        let client = rClient::new();
        let url = format!("{}/ping", welist_url);
        debug!("welist url: {}", url);
        let chain = bcinfo.chain.to_string().to_lowercase();
        let message = format!(
            "{0}{1}{2}{3}{4}",
            cfg.url, chain, bcinfo.blocks, bcinfo.median_time, bcinfo.best_block_hash
        );
        trace!("message to be sent: {}", message);
        let sign = sign_message(cfg.ssl_key_path.as_str(), &message.as_str());
        let response = client
            .post(url)
            .header("User-Agent", format!("bal-pusher/{}", VERSION))
            .json(&json!(
            {
                "url":              cfg.url,
                "chain":            chain,
                "height":           bcinfo.blocks,
                "median_time":      bcinfo.median_time,
                "last_block_hash":  bcinfo.best_block_hash,
                "signature":        sign,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            warn!(
                "Non-success response: {} {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("")
            );
        }

        let body = &(response.text().await?);
        info!("Report to welist({})\tSent: {}", welist_url, body);
    } else {
        debug!("Not sending stats");
    }
    Ok(())
}
fn sign_message(private_key_path: &str, message: &str) -> String {
    let key_data = fs::read(private_key_path).unwrap();

    let private_key = PKey::private_key_from_pem(&key_data).unwrap();
    let mut signer = Signer::new_without_digest(&private_key).unwrap();

    let signature = signer.sign_oneshot_to_vec(message.as_bytes()).unwrap();

    let signature_b64 = general_purpose::STANDARD.encode(&signature);

    signature_b64
}

fn parse_env(cfg: &mut MyConfig) {
    cfg.regtest = parse_env_netconfig(cfg, "regtest");
    cfg.signet = parse_env_netconfig(cfg, "signet");
    cfg.testnet = parse_env_netconfig(cfg, "testnet");
    cfg.testnet4 = parse_env_netconfig(cfg, "testnet4");
    drop(parse_env_netconfig(cfg, "bitcoin"));
}
fn parse_env_netconfig(cfg_lock: &mut MyConfig, chain: &str) -> NetworkParams {
    //fn parse_env_netconfig(cfg_lock: &MutexGuard<MyConfig>, chain: &str) ->  &NetworkParams{
    let cfg = match chain {
        "regtest" => &mut cfg_lock.regtest,
        "signet" => &mut cfg_lock.signet,
        "testnet" => &mut cfg_lock.testnet,
        "testnet4" => &mut cfg_lock.testnet4,
        &_ => &mut cfg_lock.mainnet,
    };
    match env::var(format!("BAL_PUSHER_{}_HOST", chain.to_uppercase())) {
        Ok(value) => {
            cfg.host = value;
        }
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_PORT", chain.to_uppercase())) {
        Ok(value) => match value.parse::<u64>() {
            Ok(value) => {
                cfg.port = value.try_into().unwrap();
            }
            Err(_) => {}
        },
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_DIR_PATH", chain.to_uppercase())) {
        Ok(value) => {
            cfg.dir_path = value;
        }
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_DB_FIELD", chain.to_uppercase())) {
        Ok(value) => {
            cfg.db_field = value;
        }
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_COOKIE_FILE", chain.to_uppercase())) {
        Ok(value) => {
            cfg.cookie_file = value;
        }
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_RPC_USER", chain.to_uppercase())) {
        Ok(value) => {
            cfg.rpc_user = value;
        }
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_RPC_PASSWORD", chain.to_uppercase())) {
        Ok(value) => {
            cfg.rpc_pass = value;
        }
        Err(_) => {}
    }
    match env::var(format!("BAL_PUSHER_{}_ZMQ_HASHBLOCK", chain.to_uppercase())) {
        Ok(value) => {
            cfg.rpc_pass = value;
        }
        Err(_) => {}
    }
    cfg.clone()
}

fn check_zmq_connection(endpoint: &str) -> bool {
    trace!("check zmq connection");
    let context = Context::new();
    let socket = match context.socket(DEALER) {
        Ok(sock) => sock,
        Err(_) => return false,
    };

    if socket.connect(endpoint).is_err() {
        return false;
    }

    // Try to send an empty message non-blocking
    socket.send("", DONTWAIT).is_ok()
}

// Add this struct to monitor connection health
struct ConnectionMonitor {
    last_message_time: Instant,
    timeout: Duration,
    consecutive_timeouts: u32,
    max_consecutive_timeouts: u32,
}

impl ConnectionMonitor {
    fn new(timeout_secs: u64, max_timeouts: u32) -> Self {
        Self {
            last_message_time: Instant::now(),
            timeout: Duration::from_secs(timeout_secs),
            consecutive_timeouts: 0,
            max_consecutive_timeouts: max_timeouts,
        }
    }

    fn update(&mut self) {
        self.last_message_time = Instant::now();
        self.consecutive_timeouts = 0;
    }

    fn check_connection(&mut self) -> ConnectionStatus {
        let elapsed = self.last_message_time.elapsed();

        if elapsed > self.timeout {
            self.consecutive_timeouts += 1;

            if self.consecutive_timeouts >= self.max_consecutive_timeouts {
                ConnectionStatus::Lost(elapsed)
            } else {
                ConnectionStatus::Warning(elapsed)
            }
        } else {
            ConnectionStatus::Healthy
        }
    }

    fn reset(&mut self) {
        self.consecutive_timeouts = 0;
        self.last_message_time = Instant::now();
    }
}

enum ConnectionStatus {
    Healthy,
    Warning(Duration),
    Lost(Duration),
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let mut cfg = MyConfig::default();

    let dbfile = env::var("BAL_PUSHER_DB_FILE").unwrap();
    parse_env(&mut cfg);
    let mut args = std::env::args();
    let _exe_name = args.next().unwrap();
    let arg_network = match args.next() {
        Some(nargs) => nargs,
        None => "bitcoin".to_string(),
    };
    let network = match arg_network.as_str() {
        "testnet" => Network::Testnet,
        "testnet4" => Network::Testnet4,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => Network::Bitcoin,
    };

    info!("Network: {}", arg_network);
    let network_params = get_network_params(&cfg, network);

    let context = Context::new();
    let socket: Socket = context.socket(zmq::SUB).unwrap();

    let zmq_address = network_params.zmq_listener.clone();
    info!("zmq listening on: {}", zmq_address);
    socket.connect(&zmq_address).unwrap();

    socket.set_subscribe(b"").unwrap();

    let _ = main_result(&cfg, network_params).await;
    info!("waiting new blocks..");
    let mut last_seq: Vec<u8> = [0; 4].to_vec();
    let mut counter = 0;
    let max = 100;
    loop {
        let message = socket.recv_multipart(0).unwrap();
        let topic = message[0].clone();
        let body = message[1].clone();
        let seq = message[2].clone();
        last_seq = seq;
        debug!(
            "ZMQ:GET TOPIC: {}",
            String::from_utf8(topic.clone()).expect("invalid topic")
        );
        trace!("ZMQ:GET BODY: {}", hex::encode(&body));
        if topic == b"hashblock" {
            info!("NEW BLOCK: {}", hex::encode(&body));
            let _ = main_result(&cfg, network_params).await;
        }
        thread::sleep(Duration::from_millis(100)); // Sleep for 100ms
    }
}
fn seq_to_str(seq: &Vec<u8>) -> String {
    if seq.len() == 4 {
        let mut rdr = Cursor::new(seq);
        let sequence = rdr
            .read_u32::<LittleEndian>()
            .expect("Failed to read integer");
        return sequence.to_string();
    }
    "Unknown".to_string()
}
