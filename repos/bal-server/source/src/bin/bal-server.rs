use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use std::env;
use std::net::IpAddr;

//use std::time::{SystemTime,UNIX_EPOCH};
use std::fs;
use std::sync::{Arc, Mutex, MutexGuard};
//use std::net::SocketAddr;
use sqlite::{Connection, State, Value};
use std::collections::HashMap;

use bitcoin::{Network, Transaction, consensus};

use chrono::Utc;
use hex_conservative::FromHex;
use log::{debug, error, info, trace};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;

use bal_server::db::{
    create_database, execute_insert, get_last_used_address_by_ip, get_next_address_index,
    insert_xpub, save_new_address,
};
use bal_server::xpub::new_address_from_xpub;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NETWORKS: [&str; 5] = ["bitcoin", "testnet", "testnet4", "signet", "regtest"];
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetConfig {
    address: String,
    fixed_fee: u64,
    xpub: bool,
    network: Network,
    name: String,
    enabled: bool,
}

impl NetConfig {
    fn default_network(name: String, network: Network) -> Self {
        NetConfig {
            address: "".to_string(),
            fixed_fee: 50000,
            xpub: false,
            name,
            network,
            enabled: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyConfig {
    regtest: NetConfig,
    signet: NetConfig,
    testnet: NetConfig,
    testnet4: NetConfig,
    mainnet: NetConfig,
    info: String,
    bind_address: String,
    bind_port: u16, // Changed to u16 for port numbers
    db_file: String,
    pub_key_path: String,
    expose_stats: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub address: String,
    pub base_fee: u64,
    pub chain: String,
    pub info: String,
    pub version: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub report_date: String,
    pub chain: String,
    pub totals: i64,
    pub waiting: i64,
    pub sent: i64,
    pub failed: i64,
    pub waiting_profit: i64,
    pub sent_profit: i64,
    pub missed_profit: i64,
    pub unique_inputs: i64,
}

impl Default for MyConfig {
    fn default() -> Self {
        MyConfig {
            regtest: NetConfig::default_network("regtest".to_string(), Network::Regtest),
            signet: NetConfig::default_network("signet".to_string(), Network::Signet),
            testnet: NetConfig::default_network("testnet".to_string(), Network::Testnet),
            testnet4: NetConfig::default_network("testnet4".to_string(), Network::Testnet4),
            mainnet: NetConfig::default_network("bitcoin".to_string(), Network::Bitcoin),
            bind_address: "127.0.0.1".to_string(),
            bind_port: 9137,
            db_file: "bal.db".to_string(),
            info: "Will Executor Server".to_string(),
            pub_key_path: "public_key.pem".to_string(),
            expose_stats: env::var("BAL_SERVER_EXPOSE_STATS")
                .unwrap_or("false".to_string())
                .parse::<bool>()
                .unwrap(),
        }
    }
}
impl MyConfig {
    fn get_net_config(&self, param: &str) -> &NetConfig {
        match param {
            "regtest" => &self.regtest,
            "testnet" => &self.testnet,
            "testnet4" => &self.testnet4,
            "signet" => &self.signet,
            _ => &self.mainnet,
        }
    }
}

async fn echo_version() -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    Ok(Response::new(full(VERSION)))
}
async fn echo_home(cfg: &MyConfig) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    debug!("echo_home: {}", cfg.info);
    Ok(Response::new(full(cfg.info.clone())))
}
async fn echo_pub_key(
    cfg: &MyConfig,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let pub_key = fs::read_to_string(&cfg.pub_key_path)
        .expect(format!("Failed to read public key file {}", cfg.pub_key_path).as_str());
    Ok(Response::new(full(pub_key)))
}
async fn echo_stats(
    param: &str,
    cfg: &MyConfig,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    info!("echo stats!!! {} - {}", param, cfg.expose_stats);
    let netconfig = MyConfig::get_net_config(cfg, param);
    if !netconfig.enabled {
        debug!("network disabled {}", param);
        return Ok(Response::new(full("network disabled")));
    }
    let sql = format!(
        "SELECT   
  report_date,
  chain,
  totals,
  waiting,
  sent,
  failed,
  waiting_profit,
  sent_profit,
  missed_profit,
  unique_inputs FROM tbl_stats where chain = '{}'
  ",
        netconfig.name
    );
    let mut stats: Vec<StatsResponse> = vec![];
    let db = sqlite::open(&cfg.db_file).unwrap();
    let _ = db.iterate(&sql, |pairs| {
        let row: HashMap<_, _> = pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.map(|s| s)))
            .collect();
        //let row:HashMap<_,_>= pairs.into_iter().collect();
        println!("row report date {}", row["report_date"].clone().unwrap());

        dbg!(&row);
        stats.push(StatsResponse {
            report_date: row["report_date"].clone().unwrap().to_string(),
            chain: row["chain"].clone().unwrap().to_string(),
            totals: row["totals"].clone().unwrap().parse::<i64>().unwrap(),
            waiting: row["waiting"].clone().unwrap().parse::<i64>().unwrap(),
            sent: row["sent"].clone().unwrap().parse::<i64>().unwrap(),
            failed: row["failed"].clone().unwrap().parse::<i64>().unwrap(),
            waiting_profit: row["waiting_profit"]
                .clone()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
            sent_profit: row["sent_profit"].clone().unwrap().parse::<i64>().unwrap(),
            missed_profit: row["missed_profit"]
                .clone()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
            unique_inputs: row["unique_inputs"]
                .clone()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
        });
        true
    });
    match serde_json::to_string(&stats) {
        Ok(json_data) => {
            debug!("echo info reply: {}", json_data);
            return Ok(Response::new(full(json_data)));
        }
        Err(err) => Ok(Response::new(full(format!("error:{}", err)))),
    }
}

async fn echo_info(
    param: &str,
    cfg: &MyConfig,
    remote_addr: &String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    info!("echo info!!!{}", param);
    let netconfig = MyConfig::get_net_config(cfg, param);
    if !netconfig.enabled {
        debug!("network disabled {}", param);
        return Ok(Response::new(full("network disabled")));
    }
    let address = match netconfig.xpub {
        false => {
            let address = netconfig.address.to_string();
            trace!("is address: {}", &address);
            address
        }
        true => {
            let db = sqlite::open(&cfg.db_file).unwrap();
            match get_last_used_address_by_ip(
                &db,
                &netconfig.name,
                &netconfig.address,
                &remote_addr,
            ) {
                Some(address) => address,
                None => {
                    let next = get_next_address_index(&db, &netconfig.name, &netconfig.address);
                    let address =
                        new_address_from_xpub(&netconfig.address, next.1, netconfig.network)
                            .unwrap();
                    save_new_address(&db, next.0, &address.0, &address.1, &remote_addr);
                    debug!("save new address {} {}", address.0, address.1);
                    trace!("next {} {}", next.0, next.1);
                    address.0
                }
            }
        }
    };
    let info = InfoResponse {
        address,
        base_fee: netconfig.fixed_fee,
        chain: netconfig.network.to_string(),
        info: cfg.info.to_string(),
        version: VERSION.to_string(),
    };
    trace!("address: {:#?}", info);
    match serde_json::to_string(&info) {
        Ok(json_data) => {
            debug!("echo info reply: {}", json_data);
            return Ok(Response::new(full(json_data)));
        }
        Err(err) => Ok(Response::new(full(format!("error:{}", err)))),
    }
}
async fn echo_search(
    whole_body: &Bytes,
    cfg: &MyConfig,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    info!("echo search!!!");
    let strbody = std::str::from_utf8(whole_body).unwrap();
    info!("{}", strbody);

    let mut response = Response::new(full("Bad data received".to_owned()));
    *response.status_mut() = StatusCode::BAD_REQUEST;
    if !strbody.is_empty() && strbody.len() <= 70 {
        let db = sqlite::open(&cfg.db_file).unwrap();
        let mut statement = db
            .prepare("SELECT * FROM tbl_tx WHERE txid = ? LIMIT 1")
            .unwrap();
        statement.bind((1, strbody)).unwrap();

        if let Ok(State::Row) = statement.next() {
            let mut response_data = HashMap::new();
            match statement.read::<String, _>("status") {
                Ok(value) => response_data.insert("status", value),
                Err(e) => {
                    error!("Error reading status: {}", e);
                    //response_data.insert("status", "Error".to_string())
                    None
                }
            };

            // Read the transaction (tx)
            match statement.read::<String, _>("tx") {
                Ok(value) => response_data.insert("tx", value),
                Err(e) => {
                    error!("Error reading tx: {}", e);
                    //response_data.insert("tx", "Error".to_string())
                    None
                }
            };

            match statement.read::<String, _>("our_address") {
                Ok(value) => response_data.insert("our_address", value),
                Err(e) => {
                    error!("Error reading address: {}", e);
                    //response_data.insert("tx", "Error".to_string())
                    None
                }
            };

            match statement.read::<String, _>("our_fees") {
                Ok(value) => response_data.insert("our_fees", value),
                Err(e) => {
                    error!("Error reading fees: {}", e);
                    //response_data.insert("tx", "Error".to_string())
                    None
                }
            };

            // Read the request id (reqid)
            match statement.read::<String, _>("reqid") {
                Ok(value) => response_data.insert("time", value),
                Err(e) => {
                    error!("Error reading reqid: {}", e);
                    //response_data.insert("time", "Error".to_string())
                    None
                }
            };
            response = match serde_json::to_string(&response_data) {
                Ok(json_data) => Response::new(full(json_data)),
                Err(_) => response,
            };

            return Ok(response);
        }
    }
    Ok(response)
}
async fn echo_push(
    whole_body: &Bytes,
    cfg: &MyConfig,
    param: &str,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    //let whole_body = req.collect().await?.to_bytes();
    trace!("echo_push");
    let strbody = std::str::from_utf8(whole_body).unwrap();
    let mut response = Response::new(full("Bad data received".to_owned()));
    let mut response_not_enable = Response::new(full("Network not enabled".to_owned()));
    *response.status_mut() = StatusCode::BAD_REQUEST;
    *response_not_enable.status_mut() = StatusCode::BAD_REQUEST;
    let netconfig = MyConfig::get_net_config(cfg, param);
    if !netconfig.enabled {
        trace!("network not enabled {}", &netconfig.name);
        return Ok(response_not_enable);
    }
    let req_time = Utc::now().timestamp_nanos_opt().unwrap(); // Returns i64

    let db = sqlite::open(&cfg.db_file).unwrap();

    let lines = strbody.split("\n");
    let sqltxshead = "INSERT INTO tbl_tx (txid, wtxid, ntxid, tx, locktime, reqid, network, our_address, our_fees)".to_string();
    let mut sqltxs = "".to_string();
    let sqlinpshead = "INSERT INTO tbl_inp (txid, in_txid, in_vout )".to_string();
    let mut sqlinps = "".to_string();
    let sqloutshead = "INSERT INTO tbl_out (txid, vout, script_pubkey, amount )".to_string();
    let mut sqlouts = "".to_string();
    let mut union_tx = true;
    let mut union_inps = true;
    let mut union_outs = true;
    let mut already_present = false;
    let mut ptx: Vec<(usize, Value)> = vec![];
    let mut pinps: Vec<(usize, Value)> = vec![];
    let mut pouts: Vec<(usize, Value)> = vec![];
    let mut linenum = 1;
    let mut lineinp = 1;
    let mut lineout = 1;
    for line in lines {
        if line.is_empty() {
            trace!("line len is: {}", line.len());
            continue;
        }
        let linea = format!("{req_time}:{line}");
        info!("New Tx: {}", linea);
        let raw_tx = match Vec::<u8>::from_hex(line) {
            Ok(raw_tx) => raw_tx,
            Err(err) => {
                error!("rawtx error: {}", err);
                continue;
            }
        };
        if !raw_tx.is_empty() {
            trace!("len: {}", raw_tx.len());
            let tx: Transaction = match consensus::deserialize(&raw_tx) {
                Ok(tx) => tx,
                Err(err) => {
                    error!("error: unable to parse tx: {}\n{}", line, err);
                    continue;
                }
            };
            let txid = tx.compute_txid().to_string();
            trace!("txid: {}", txid);
            let mut statement = db.prepare("SELECT * FROM tbl_tx WHERE txid = ?").unwrap();
            statement.bind((1, &txid[..])).unwrap();
            if let Ok(State::Row) = statement.next() {
                trace!("already present");
                already_present = true;
                continue;
            }
            let ntxid = tx.compute_ntxid();
            let wtxid = tx.compute_wtxid();
            let mut found = false;
            let locktime = tx.lock_time;
            let mut our_address: String = "".to_string();
            let mut our_fees: u64 = 0;
            for input in tx.input {
                if !union_inps {
                    sqlinps = format!("{sqlinps} UNION ALL");
                } else {
                    union_inps = false;
                }
                sqlinps = format!("{sqlinps} SELECT ?, ?, ?");
                pinps.push((lineinp, Value::String(txid.to_string())));
                pinps.push((
                    lineinp + 1,
                    Value::String(input.previous_output.txid.to_string()),
                ));
                pinps.push((
                    lineinp + 2,
                    Value::String(input.previous_output.vout.to_string()),
                ));
                lineinp += 3;
            }
            if netconfig.fixed_fee == 0 {
                found = true;
            }
            for (idx, output) in tx.output.into_iter().enumerate() {
                let script_pubkey = output.script_pubkey;
                let address = match bitcoin::Address::from_script(
                    script_pubkey.as_script(),
                    netconfig.network,
                ) {
                    Ok(address) => address.to_string(),
                    Err(_) => String::new(),
                };
                let amount = output.value;
                our_fees = netconfig.fixed_fee; //search wllexecutor output
                if netconfig.xpub {
                    let sql = "select * from tbl_address where address=?";
                    let mut stmt = db.prepare(sql).expect("failed to fetch addresses");
                    stmt.bind((1, Value::String(address.to_string()))).unwrap();
                    if let Ok(State::Row) = stmt.next() {
                        our_address = address.to_string();
                    }
                } else {
                    our_address = netconfig.address.to_string();
                }
                if address == our_address && amount.to_sat() >= netconfig.fixed_fee {
                    our_fees = amount.to_sat();
                    //our_address = netconfig.address.to_string();
                    found = true;
                    trace!("address and fees are correct {}: {}", our_address, our_fees);
                }
                if !union_outs {
                    sqlouts = format!("{sqlouts} UNION ALL");
                } else {
                    union_outs = false;
                }
                sqlouts = format!("{sqlouts} SELECT ?, ?, ?, ?");
                pouts.push((lineout, Value::String(txid.to_string())));
                pouts.push((lineout + 1, Value::Integer(idx.try_into().unwrap())));
                pouts.push((lineout + 2, Value::String(script_pubkey.to_string())));
                pouts.push((
                    lineout + 3,
                    Value::Integer(amount.to_sat().try_into().unwrap()),
                ));
                lineout += 4;
            }
            if !found {
                error!("willexecutor output not found ");
                return Ok(response);
            } else {
                if !union_tx {
                    sqltxs = format!("{sqltxs} UNION ALL");
                } else {
                    union_tx = false;
                }
                sqltxs = format!("{sqltxs}  SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?");
                ptx.push((linenum, Value::String(txid)));
                ptx.push((linenum + 1, Value::String(wtxid.to_string())));
                ptx.push((linenum + 2, Value::String(ntxid.to_string())));
                ptx.push((linenum + 3, Value::String(line.to_string())));
                ptx.push((linenum + 4, Value::String(locktime.to_string())));
                ptx.push((linenum + 5, Value::String(req_time.to_string())));
                ptx.push((linenum + 6, Value::String(netconfig.name.to_string())));
                ptx.push((linenum + 7, Value::String(our_address.to_string())));
                ptx.push((linenum + 8, Value::String(our_fees.to_string())));
                linenum += 9;
            }
        } else {
            trace!("rawTx len is: {}", raw_tx.len());
            debug!("{}", &sqltxs);
        }
    }
    if sqltxs.is_empty() && already_present {
        return Ok(Response::new(full("already present")));
    }
    let sqltxs = format!("{}{};", sqltxshead, sqltxs);
    let sqlinps = format!("{}{};", sqlinpshead, sqlinps);
    let sqlouts = format!("{}{};", sqloutshead, sqlouts);
    if let Err(err) = execute_insert(&db, sqltxs, ptx, sqlinps, pinps, sqlouts, pouts) {
        debug!("{}", err);
        return Ok(response);
    }
    Ok(Response::new(full("thx")))
}

fn match_uri<'a>(path: &str, uri: &'a str) -> Option<&'a str> {
    let re = Regex::new(path).unwrap();
    if let Some(captures) = re.captures(uri) {
        if let Some(param) = captures.name("param") {
            return Some(param.as_str());
        }
    }
    None
}

async fn echo(
    req: Request<hyper::body::Incoming>,
    cfg: &MyConfig,
    ip: &String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let mut not_found = Response::new(empty());
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    let mut ret: Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> = Ok(not_found);

    let uri = req.uri().path().to_string();

    let remote_addr = req
        .headers()
        .get("X-Real-IP")
        .and_then(|value| value.to_str().ok())
        .and_then(|xff| xff.split(',').next())
        .map(|ip| ip.trim().to_string())
        .unwrap_or_else(|| ip.to_string());
    trace!("{}: {}", remote_addr, uri);
    match *req.method() {
        // Serve some instructions at /
        Method::POST => {
            let whole_body = req.collect().await?.to_bytes();
            if let Some(param) = match_uri(r"^?/?(?P<param>[^/]?+)?/pushtxs$", uri.as_str()) {
                //let whole_body = collect_body(req,512_000).await?;
                ret = echo_push(&whole_body, cfg, param).await;
            }
            if uri == "/searchtx" {
                //let whole_body = collect_body(req,64).await?;
                ret = echo_search(&whole_body, cfg).await;
            }
            ret
        }
        Method::GET => {
            if let Some(param) = match_uri(r"^?/?(?P<param>[^/]?+)?/stats$", uri.as_str()) {
                ret = echo_stats(param, cfg).await;
            }
            if let Some(param) = match_uri(r"^?/?(?P<param>[^/]?+)?/info$", uri.as_str()) {
                ret = echo_info(param, cfg, &remote_addr).await;
            }
            if uri == "/version" {
                ret = echo_version().await;
            }
            if uri == "/.pub_key.pem" {
                ret = echo_pub_key(cfg).await;
            }
            if uri == "/" {
                ret = echo_home(cfg).await;
            }
            ret
        }

        // Return the 404 Not Found for other routes.
        _ => ret,
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
fn parse_env(cfg: &Arc<Mutex<MyConfig>>) {
    //for (key, value) in std::env::vars() {
    //    debug!("ENVIRONMENT {key}: {value}");
    //}
    let mut cfg_lock = cfg.lock().unwrap();
    if let Ok(value) = env::var("BAL_SERVER_DB_FILE") {
        debug!("BAL_SERVER_DB_FILE: {}", value);
        cfg_lock.db_file = value;
    }
    if let Ok(value) = env::var("BAL_SERVER_BIND_ADDRESS") {
        debug!("BAL_SERVER_BIND_ADDRESS: {}", value);
        cfg_lock.bind_address = value;
    }
    if let Ok(value) = env::var("BAL_SERVER_BIND_PORT") {
        debug!("BAL_SERVER_BIND_PORT: {}", value);
        if let Ok(v) = value.parse::<u16>() {
            cfg_lock.bind_port = v;
        }
    }

    if let Ok(value) = env::var("BAL_SERVER_PUB_KEY_PATH") {
        debug!("BAL_SERVER_PUB_KEY_PATH: {}", value);
        cfg_lock.pub_key_path = value;
    }

    if let Ok(value) = env::var("BAL_SERVER_INFO") {
        debug!("BAL_SERVER_INFO: {}", value);
        cfg_lock.info = value;
    }
    cfg_lock = parse_env_netconfig(cfg_lock, "regtest");
    cfg_lock = parse_env_netconfig(cfg_lock, "signet");
    cfg_lock = parse_env_netconfig(cfg_lock, "testnet");
    cfg_lock = parse_env_netconfig(cfg_lock, "testnet4");
    drop(parse_env_netconfig(cfg_lock, "bitcoin"));
}
fn parse_env_netconfig<'a>(
    mut cfg_lock: MutexGuard<'a, MyConfig>,
    chain: &'a str,
) -> MutexGuard<'a, MyConfig> {
    let cfg = match chain {
        "regtest" => &mut cfg_lock.regtest,
        "signet" => &mut cfg_lock.signet,
        "testnet" => &mut cfg_lock.testnet,
        "testnet4" => &mut cfg_lock.testnet4,
        &_ => &mut cfg_lock.mainnet,
    };
    if let Ok(value) = env::var(format!("BAL_SERVER_{}_ADDRESS", chain.to_uppercase())) {
        debug!("BAL_SERVER_{}_ADDRESS: {}", chain.to_uppercase(), value);
        cfg.address = value;
        if cfg.address.len() > 5 {
            if cfg.address[1..4] == *"pub" {
                cfg.xpub = true;
                trace!("is_xpub");
            }
            cfg.enabled = true;
        }
    }

    if let Ok(value) = env::var(format!("BAL_SERVER_{}_FIXED_FEE", chain.to_uppercase())) {
        debug!("BAL_SERVER_{}_FIXED_FEE: {}", chain.to_uppercase(), value);
        if let Ok(v) = value.parse::<u64>() {
            cfg.fixed_fee = v;
        }
    }
    cfg_lock
}

fn init_network(db: &Connection, cfg: &MyConfig) {
    for network in NETWORKS {
        let netconfig = MyConfig::get_net_config(cfg, network);
        insert_xpub(db, &netconfig.name, &netconfig.address);
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let cfg: Arc<Mutex<MyConfig>> = Arc::<Mutex<MyConfig>>::default();
    parse_env(&cfg);

    let cfg_lock = cfg.lock().unwrap();

    let db = sqlite::open(&cfg_lock.db_file).unwrap();
    create_database(&db);
    init_network(&db, &cfg_lock);

    let addr = cfg_lock.bind_address.to_string();
    let addr: IpAddr = addr.parse()?;

    let listener = TcpListener::bind((addr, cfg_lock.bind_port)).await?;
    info!("Listening on http://{}:{}", addr, cfg_lock.bind_port);

    loop {
        let (stream, _) = listener.accept().await?;
        let ip = stream
            .peer_addr()?
            .to_string()
            .split(":")
            .next()
            .unwrap()
            .to_string();
        let io = TokioIo::new(stream);

        tokio::task::spawn({
            let cfg = cfg_lock.clone();
            async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(|req: Request<hyper::body::Incoming>| async {
                            echo(req, &cfg, &ip).await
                        }),
                    )
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            }
        });
    }
}
