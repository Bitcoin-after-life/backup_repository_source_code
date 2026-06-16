extern crate bitcoincore_rpc;
extern crate zmq;
use bitcoin::Network;

use bitcoincore_rpc::{bitcoin, Auth, Client, Error, RpcApi};
use bitcoincore_rpc_json::GetBlockchainInfoResult;

use sqlite::{Value};
use serde::Serialize;
use serde::Deserialize;
use std::env;
use std::fs::OpenOptions;
use std::io::{ Write};
use log::{info,debug,warn,error};
use zmq::{Context, Socket};
use std::str;
use std::{thread, time::Duration};
use std::collections::HashMap;
//use byteorder::{LittleEndian, ReadBytesExt};
//use std::io::Cursor;
use hex;
use std::error::Error as StdError;

const LOCKTIME_THRESHOLD:i64 = 5000000;

#[derive(Debug, Clone,Serialize, Deserialize)]
struct MyConfig {
    zmq_listener:   String,
    requests_file:  String,
    db_file:        String,
    bitcoin_dir:    String,
    regtest:        NetworkParams,
    testnet:        NetworkParams,
    signet:         NetworkParams,
    mainnet:        NetworkParams,


}

impl Default for MyConfig {
    fn default() -> Self {
        MyConfig {
            zmq_listener:   "tcp://127.0.0.1:28332".to_string(),
            requests_file:  "rawrequests.log".to_string(),
            db_file:        "../bal.db".to_string(),
            bitcoin_dir:    "".to_string(),
            regtest:        get_network_params_default(Network::Regtest),
            testnet:        get_network_params_default(Network::Testnet),
            signet:         get_network_params_default(Network::Signet),
            mainnet:        get_network_params_default(Network::Bitcoin),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkParams {
    host:           String,
    port:           u16,
    dir_path:       String,
    db_field:       String,
    cookie_file:    String,
    rpc_user:       String,
    rpc_pass:       String,
}
fn get_network_params(cfg: &MyConfig,network:Network)-> &NetworkParams{
    match network{
        Network::Testnet => &cfg.testnet,
        Network::Signet => &cfg.signet,
        Network::Regtest => &cfg.regtest,
        _ => &cfg.mainnet
    }
}
fn get_network_params_default(network:Network) -> NetworkParams{
    match network {
        Network::Testnet    =>  NetworkParams{
            host:           "http://localhost".to_string(),
            port:           18332,
            dir_path:       "testnet3/".to_string(),
            db_field:       "testnet".to_string(),
            cookie_file:    "".to_string(),
            rpc_user:       "".to_string(),
            rpc_pass:       "".to_string(),
        },
        Network::Signet     =>  NetworkParams{
            host:           "http://localhost".to_string(),
            port:           18332,
            dir_path:       "signet/".to_string(),
            db_field:        "signet".to_string(),
            cookie_file:    "".to_string(),
            rpc_user:       "".to_string(),
            rpc_pass:       "".to_string(),
        },
        Network::Regtest    =>  NetworkParams{
            host:           "http://localhost".to_string(),
            port:           18443,
            dir_path:       "regtest/".to_string(),
            db_field:        "regtest".to_string(),
            cookie_file:    "".to_string(),
            rpc_user:       "".to_string(),
            rpc_pass:       "".to_string(),
        },
        _                   =>  NetworkParams{
            host:           "http://localhost".to_string(),
            port:           8332,
            dir_path:       "".to_string(),
            db_field:       "bitcoin".to_string(),
            cookie_file:    "".to_string(),
            rpc_user:       "".to_string(),
            rpc_pass:       "".to_string(),
        },
    }
}

fn get_cookie_filename(network: &NetworkParams) ->Result<String,Box<dyn StdError>>{
    if network.cookie_file !=""{
        Ok(network.cookie_file.clone())
    }else{
        match env::var_os("HOME") {
            Some(home) => {
                info!("some home {}",home.to_str().unwrap());
                match home.to_str(){
                    Some(home_str) => {
                        let cookie_file_path = format!("{}/.bitcoin/{}.cookie",home_str, network.dir_path);
                        
                        Ok(cookie_file_path)
                    },
                    None => Err("wrong HOME value".into())
                }
            },
            None => Err("Please Set HOME environment variable".into())
        }
    }
}
fn get_client_from_username(url: &String, network: &NetworkParams) -> Result<(Client,GetBlockchainInfoResult),Box<dyn StdError>>{
    if network.rpc_user != "" {
        match Client::new(&url[..],Auth::UserPass(network.rpc_user.to_string(),network.rpc_pass.to_string())){
            Ok(client) => match client.get_blockchain_info(){
                Ok(bcinfo) => Ok((client,bcinfo)),
                Err(err) => Err(err.into())
             }
            Err(err)=>Err(err.into())
        }
    }else{
        Err("Failed".into())
    }
}
fn get_client_from_cookie(url: &String,network: &NetworkParams)->Result<(Client,GetBlockchainInfoResult),Box<dyn StdError>>{
    match get_cookie_filename(network){
        Ok(cookie) => {
            match Client::new(&url[..], Auth::CookieFile(cookie.into())) {
                Ok(client) => match client.get_blockchain_info(){
                    Ok(bcinfo) => Ok((client,bcinfo)),
                    Err(err) => Err(err.into())
                },
                Err(err)=>Err(err.into())

            }
        },
        Err(err)=>Err(err.into())
    }
}
fn get_client(network: &NetworkParams) -> Result<(Client,GetBlockchainInfoResult),Box<dyn StdError>>{
    let url = format!("{}:{}",network.host,&network.port);
    match get_client_from_username(&url,network){
        Ok(client) =>{Ok(client)},
        Err(_) =>{
            match get_client_from_cookie(&url,&network){
                Ok(client)=>{
                    Ok(client)
                },
                Err(err)=> Err(err.into())
            }
        }
    }
}
fn main_result(cfg: &MyConfig, network_params: &NetworkParams) -> Result<(), Error> {


    /*let url = args.next().expect("Usage: <rpc_url> <username> <password>");
    let user = args.next().expect("no user given");
    let pass = args.next().expect("no pass given");
    */
    //let network = Network::Regtest
    match get_client(network_params){
        Ok((rpc,bcinfo)) => {
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
            info!("median time: {}",bcinfo.median_time);
            let average_time = bcinfo.median_time;
            let db = sqlite::open(&cfg.db_file).unwrap();
            
            let query_tx = db.prepare("SELECT  * FROM tbl_tx WHERE network = :network AND status = :status AND ( locktime < :bestblock_height  OR locktime > :locktime_threshold AND locktime < :bestblock_time);").unwrap().into_iter();
            //let query_tx = db.prepare("SELECT * FROM tbl_tx where status = :status").unwrap().into_iter();
            let mut pushed_txs:Vec<String> = Vec::new();
            let mut invalid_txs: std::collections::HashMap<String, String> = HashMap::new();
            for row in query_tx.bind::<&[(_, Value)]>(&[
                (":locktime_threshold", (LOCKTIME_THRESHOLD as i64).into()),
                (":bestblock_time", (average_time as i64).into()),
                (":bestblock_height", (bcinfo.blocks as i64).into()),
                (":network", network_params.db_field.clone().into()),
                (":status", 0.into()),
                ][..])
            .unwrap()
            .map(|row| row.unwrap())
            {
                let tx = row.read::<&str, _>("tx");
                let txid = row.read::<&str, _>("txid");
                let locktime = row.read::<i64,_>("locktime");
                info!("to be pushed: {}: {}",txid, locktime);
                match rpc.send_raw_transaction(tx){
                    Ok(o) => {
                        let mut file = OpenOptions::new()
                            .append(true) // Set the append option
                            .create(true) // Create the file if it doesn't exist
                            .open("valid_txs")?;
                        let data = format!("{}\t:\t{}\t:\t{}\n",txid,average_time,locktime);
                        file.write_all(data.as_bytes())?;
                        drop(file);

                        info!("tx: {} pusshata PUSHED\n{}",txid,o);
                        pushed_txs.push(txid.to_string());
                    },
                    Err(err) => {
                        let mut file = OpenOptions::new()
                            .append(true) // Set the append option
                            .create(true) // Create the file if it doesn't exist
                            .open("invalid_txs")?;
                        let data = format!("{}:\t{}\t:\t{}\t:\t{}\n",txid,err,average_time,locktime);
                        file.write_all(data.as_bytes())?;
                        drop(file);
                        warn!("Error: {}\n{}",err,txid);
                        //store err in invalid_txs
                        invalid_txs.insert(txid.to_string(), err.to_string());

                    },
                };
            }
            
            if pushed_txs.len() > 0 {
                let _ = db.execute(format!("UPDATE tbl_tx SET status = 1 WHERE txid in ('{}');",pushed_txs.join("','")));
            }
            if invalid_txs.len() > 0 {
                for (txid,txerr) in &invalid_txs{
                    //let _ = db.execute(format!("UPDATE tbl_tx SET status = 2 WHERE txid in ('{}'Yp);",invalid_txs.join("','")));
                    let _ = db.execute(format!("UPDATE tbl_tx SET status = 2, push_err='{txerr}' WHERE txid = '{txid}'"));
                }
            }
        }
        Err(_)=>{
            panic!("impossible to get client")
        }
    }
    Ok(())
}

fn parse_env(cfg: &mut MyConfig){
    match env::var("BAL_PUSHER_ZMQ_LISTENER") {
        Ok(value) => {
            cfg.zmq_listener = value;},
        Err(_) => {},
    }
    match env::var("BAL_PUSHER_REQUEST_FILE") {
        Ok(value) => {
            cfg.requests_file = value;},
        Err(_) => {},
    }
    match env::var("BAL_PUSHER_DB_FILE") {
        Ok(value) => {
            cfg.db_file = value;},
        Err(_) => {},
    }
    match env::var("BAL_PUSHER_BITCOIN_DIR") {
        Ok(value) => {
            cfg.bitcoin_dir = value;},
        Err(_) => {},
    }
    cfg.regtest = parse_env_netconfig(cfg,"regtest");
    cfg.signet = parse_env_netconfig(cfg,"signet");
    cfg.testnet = parse_env_netconfig(cfg,"testnet");
    drop(parse_env_netconfig(cfg,"bitcoin"));

}
fn parse_env_netconfig(cfg_lock: &mut MyConfig, chain: &str) ->  NetworkParams{
//fn parse_env_netconfig(cfg_lock: &MutexGuard<MyConfig>, chain: &str) ->  &NetworkParams{
    let cfg = match chain{
        "regtest" => &mut cfg_lock.regtest,
        "signet" => &mut cfg_lock.signet,
        "testnet" => &mut cfg_lock.testnet,
        &_ => &mut cfg_lock.mainnet,
    };
    match env::var(format!("BAL_PUSHER_{}_HOST",chain.to_uppercase())) {
        Ok(value) => { cfg.host= value; },
        Err(_) => {},
    }
    match env::var(format!("BAL_PUSHER_{}_PORT",chain.to_uppercase())) {
        Ok(value) => {
            match value.parse::<u64>(){
                Ok(value) =>{ cfg.port = value.try_into().unwrap(); },
                Err(_) => {},
            }
        }
        Err(_) => {},
    }
    match env::var(format!("BAL_PUSHER_{}_DIR_PATH",chain.to_uppercase())) {
        Ok(value) => { cfg.dir_path = value; },
        Err(_) => {},
    }
    match env::var(format!("BAL_PUSHER_{}_DB_FIELD",chain.to_uppercase())) {
        Ok(value) => { cfg.db_field = value; },
        Err(_) => {},
    }
    match env::var(format!("BAL_PUSHER_{}_COOKIE_FILE",chain.to_uppercase())) {
        Ok(value) => { 
            cfg.cookie_file = value; },
        Err(_) => {},
    }
    match env::var(format!("BAL_PUSHER_{}_RPC_USER",chain.to_uppercase())) {
        Ok(value) => { cfg.rpc_user = value; },
        Err(_) => {},
    }
    match env::var(format!("BAL_PUSHER_{}_RPC_PASSWORD",chain.to_uppercase())) {
        Ok(value) => { cfg.rpc_pass = value; },
        Err(_) => {},
    }
    cfg.clone()
}

fn get_default_config()-> MyConfig {
    let file = confy::get_configuration_file_path("bal-pusher",None).expect("Error while getting path");
    info!("Default configuration file path is: {:#?}", file);
    confy::load("bal-pusher",None).expect("cant_load")
}

fn main(){
    env_logger::init();
    let mut cfg: MyConfig = match env::var("BAL_PUSHER_CONFIG_FILE") {
        Ok(value) => {
                match confy::load_path(value.to_string()){
                    Ok(val) => {
                        info!("The configuration file path is: {:#?}", value);
                        val
                    },
                    Err(err) => {
                        error!("{}",err);
                        get_default_config()
                    }
                }
        },
        Err(_) => {
            get_default_config()
        },
    };

    parse_env(&mut cfg);
    let mut args = std::env::args();
    let _exe_name = args.next().unwrap();
    let arg_network = match args.next(){
        Some(nargs) => nargs,
        None => "bitcoin".to_string()
    };
    let network = match arg_network.as_str(){

        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => Network::Bitcoin,
    };


    debug!("Network: {}",arg_network);
    let network_params = get_network_params(&cfg,network);


    let context = Context::new();
    let socket: Socket = context.socket(zmq::SUB).unwrap();

    let zmq_address = cfg.zmq_listener.clone();
    socket.connect(&zmq_address).unwrap();

    socket.set_subscribe(b"").unwrap(); 

    let _ = main_result(&cfg,&network_params);
    info!("waiting new blocks..");
    let mut last_seq:Vec<u8>=[0;4].to_vec();
    loop {
        let message = socket.recv_multipart(0).unwrap();
        let topic = message[0].clone();
        let body = message[1].clone();
        let seq = message[2].clone();
        if last_seq >= seq {
            continue
        }
        last_seq = seq;
        //let mut sequence_str = "Unknown".to_string();
        /*if seq.len()==4{
            let mut rdr = Cursor::new(seq);
            let sequence = rdr.read_u32::<LittleEndian>().expect("Failed to read integer");
            sequence_str = sequence.to_string();
        }*/
        if topic == b"hashblock" {
            info!("NEW BLOCK{}", hex::encode(body));  
            //let cfg = cfg.clone();
            let _ = main_result(&cfg,&network_params);
        }
        thread::sleep(Duration::from_millis(100)); // Sleep for 100ms
    }
}
