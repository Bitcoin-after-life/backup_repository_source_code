use log::{error, info, trace};
use sqlite::{Connection, Error, State, Value};

pub fn create_database(db: &Connection) {
    info!("database sanity check");
    let _ = db.execute("CREATE TABLE IF NOT EXISTS tbl_tx      (txid PRIMARY KEY, date_creation TIMESTAMP DEFAULT CURRENT_TIMESTAMP, date_update TIMESTAMP DEFAULT CURRENT_TIMESTAMP, wtxid, ntxid, tx, locktime integer, network, network_fees, reqid, our_fees, our_address, status integer DEFAULT 0);");
    let _ = db.execute("ALTER TABLE tbl_tx ADD COLUMN push_err TEXT");

    let _ = db.execute("CREATE TABLE IF NOT EXISTS tbl_inp(id, txid, in_txid, in_vout);");
    let _ = db.execute("CREATE UNIQUE INDEX ON tbl_inp(txid,in_txid,in_vout);");

    let _ =
        db.execute("CREATE TABLE IF NOT EXISTS tbl_out(id, txid, script_pubkey, amount, vout);");
    let _ = db.execute("CREATE UNIQUE INDEX ON tbl_out(txid, script_pubkey, amount, vout);");

    let _ = db.execute("CREATE TABLE IF NOT EXISTS tbl_xpub (id INTEGER PRIMARY KEY , network TEXT, xpub TEXT, date_create TIMESTAMP DEFAULT CURRENT_TIMESTAMP,path_idx INTEGER DEFAULT -1);");
    let _ = db.execute("CREATE UNIQUE INDEX idx_xpub ON tbl_xpub (network, xpub)");
    let _ = db.execute("CREATE TABLE IF NOT EXISTS tbl_address (address TEXT PRIMARY_KEY, path TEXT NOT NULL, date_create TIMESTAMP DEFAULT CURRENT_TIMESTAMP, xpub INTEGER,remote_address TEXT);");

    let _ = db.execute("UPDATE tbl_tx set network='bitcoin' where network='mainnet');");
}
/*
 pub fn get_xpub_id(db: &Connection, network: &String, xpub: &String) -> Option<i64>{
    let mut stmt = db.prepare("SELECT * FROM tbl_xpub where network = ? and xpub = ?;").unwrap();
    let _ = stmt.bind((1,Value::String(network.to_string()))).unwrap();
    let _ = stmt.bind((2,Value::String(xpub.to_string()))).unwrap();
    if let  Ok(State::Row) = stmt.next(){
        return Some(stmt.read::<i64, _>("id").unwrap());
    } else {
        return None;
    }
}
*/
pub fn insert_xpub(db: &Connection, network: &String, xpub: &String) {
    if xpub != "" {
        trace!("going to insert: {} xpub:{}", network, xpub);
        let mut stmt = db
            .prepare("INSERT INTO tbl_xpub(network,xpub) VALUES(?, ?);")
            .unwrap();
        let _ = stmt.bind((1, Value::String(network.to_string()))).unwrap();
        let _ = stmt.bind((2, Value::String(xpub.to_string()))).unwrap();
        let _ = stmt.next();
    }
}

pub fn get_last_used_address_by_ip(
    db: &Connection,
    network: &String,
    xpub: &String,
    address: &String,
) -> Option<String> {
    let mut stmt = db.prepare("SELECT tbl_address.address FROM tbl_xpub join tbl_address on(tbl_xpub.id = tbl_address.xpub) where tbl_xpub.network = ? and tbl_address.remote_address = ? and tbl_xpub.xpub = ? ORDER BY tbl_address.date_create DESC LIMIT 1;").unwrap();
    let _ = stmt.bind((1, Value::String(network.to_string())));
    let _ = stmt.bind((2, Value::String(address.to_string())));
    let _ = stmt.bind((3, Value::String(xpub.to_string())));
    if let Ok(State::Row) = stmt.next() {
        let address = stmt.read::<String, _>("address").unwrap();
        return Some(address);
    } else {
        return None;
    }
}
pub fn get_next_address_index(db: &Connection, network: &String, xpub: &String) -> (i64, i64) {
    let mut stmt = db.prepare("UPDATE tbl_xpub SET path_idx = path_idx + 1 WHERE network = ? and xpub= ?  RETURNING path_idx,id;").unwrap();
    stmt.bind((1, Value::String(network.to_string()))).unwrap();
    stmt.bind((2, Value::String(xpub.to_string()))).unwrap();
    match stmt.next() {
        Ok(State::Row) => {
            let next = stmt.read::<i64, _>("path_idx").unwrap();
            let id = stmt.read::<i64, _>("id").unwrap();
            return (id, next);
        }
        Err(_) => {
            return (0, 0);
        }
        Ok(State::Done) => {
            return (0, 0);
        }
    };
}
pub fn save_new_address(
    db: &Connection,
    xpub: i64,
    address: &String,
    path: &String,
    remote_addr: &String,
) {
    let mut stmt = db
        .prepare("INSERT INTO tbl_address(address,path,xpub,remote_address) VALUES(?,?,?,?);")
        .unwrap();

    stmt.bind((1, Value::String(address.to_string()))).unwrap();
    stmt.bind((2, Value::String(path.to_string()))).unwrap();
    stmt.bind((3, Value::Integer(xpub))).unwrap();
    stmt.bind((4, Value::String(remote_addr.to_string())))
        .unwrap();

    let _ = stmt.next();
}
pub fn execute_insert(
    db: &Connection,
    sqltxs: String,
    ptx: Vec<(usize, Value)>,
    sqlinp: String,
    pinp: Vec<(usize, Value)>,
    sqlout: String,
    pout: Vec<(usize, Value)>,
) -> Result<(), Error> {
    let _ = db.execute("BEGIN TRANSACTION");
    let mut stmt = db
        .prepare(sqltxs.as_str())
        .expect("failed to prepare sqltxs");
    if let Err(err) = stmt.bind::<&[(_, Value)]>(&ptx[..]) {
        error!("error binding transaction parameters: {}", err);
        let _ = db.execute("ROLLBACK");
        return Err(err);
    }
    if let Err(err) = stmt.next() {
        error!("error inserting transactions {}", err);
        let _ = db.execute("ROLLBACK");
    } else {
        let mut stmt = db
            .prepare(sqlinp.as_str())
            .expect("failed to prepare sqlinp");
        if let Err(err) = stmt.bind::<&[(_, Value)]>(&pinp[..]) {
            error!("error binding inputs parameters {}", err);
            let _ = db.execute("ROLLBACK");
            return Err(err);
        }
        if let Err(err) = stmt.next() {
            error!("error inserting inputs {}", err);
            let _ = db.execute("ROLLBACK");
            return Err(err);
        } else {
            let mut stmt = db
                .prepare(sqlout.as_str())
                .expect("failed to prepare sqlout");
            if let Err(err) = stmt.bind::<&[(_, Value)]>(&pout[..]) {
                error!("error binding outs parameters {}", err);
                let _ = db.execute("ROLLBACK");
                return Err(err);
            }
            if let Err(err) = stmt.next() {
                error!("error inserting outs {}", err);
                let _ = db.execute("ROLLBACK");
                return Err(err);
            }
        }
    }
    let _ = db.execute("COMMIT");
    Ok(())
}
pub fn get_total_transaction_number(db: Connection, network: &String) -> Result<i64, Error> {
    let mut stmt = db
        .prepare("SELECT COUNT(*) as total_number FROM tbl_tx where network = ?;")
        .unwrap();
    stmt.bind((1, Value::String(network.to_string()))).unwrap();
    match stmt.next() {
        Ok(State::Row) => Ok(stmt.read::<i64, _>("total_number").unwrap()),
        Ok(sqlite::State::Done) => todo!(),
        Err(err) => Err(err),
    }
}
