use std;

use rmp_serde;
use serde::Serialize;

use filestorage;
use filestorage::errors::*;
use filestorage::util::*;

use msgparse::{Zeo, ZeoIter};

macro_rules! encode {
    ($data: expr) => (
        {
            let mut buf: Vec<u8> = vec![];
            {
                let mut encoder = rmp_serde::Serializer::new(&mut buf);
                ($data).serialize(&mut encoder)
            }.and(Ok(buf)).chain_err(|| "encode")
        }
    )
}

macro_rules! response {
    ($id: expr, $data: expr) => (
        try!(encode!(($id, "R", ($data))))
    )
}

macro_rules! respond {
    ($sender: expr, $id: expr, $data: expr) => (
        try!($sender.send(Zeo::Raw(response!($id, $data)))
             .chain_err(|| "send response"))
    )
}

macro_rules! error_response {
    ($id: expr, $data: expr) => (
        try!(encode!(($id, "E", ($data))))
    )
}

macro_rules! error_respond {
    ($sender: expr, $id: expr, $data: expr) => (
        try!($sender.send(Zeo::Raw(error_response!($id, $data)))
             .chain_err(|| "send error response"))
    )
}

const NIL: Option<u32> = None;

fn reader(fs: Arc<filestorage::FileStorage>,
          stream: std::net::TcpStream,
          sender: std::sync::mpsc::Sender<Zeo>)
          -> Result<()> {

    let mut it = ZeoIter::new(stream);

    // handshake
    if try!(it.next_vec()) != b"M5".to_vec() {
        return Err("Bad handshake".into())
    }

    // register(storage_id, read_only)
    loop {
        match try!(it.next()) {
            Zeo::Register(id, storage, read_only) => {
                if &storage != "1" {
                    sender.send(Zeo::Error(
                        id, "builtins.ValueError", "Invalid storage"));
                }
                respond!(sender, id, fs.last_transaction());
                break;          // onward
            },
            Zeo::LoadBefore(id, oid, before) => {
                use filestorage::LoadBeforeResult::*;
                match try!(fs.load_before(&oid, &before)) {
                    Loaded(data, tid, Some(end)) => {
                            respond!(sender, id, (data, tid, end));
                        },
                    Loaded(data, tid, None) => {
                        respond!(sender, id, (data, tid, NIL));
                    },
                    NoneBefore => {
                        respond!(sender, id, NIL);
                    },
                    PosKeyError => {
                        error_respond!(
                            sender, id,
                            ("ZODB.POSException.POSKeyError", (oid,)));
                    },
                }
            },
            Zeo::End => {
                sender.send(Zeo::End);
                return Ok(())
            },
            _ => return Err("bad method".into())
        }
    }

    // Main loop. We spend most of our time here.
    loop {
        match try!(it.next()) {
            Zeo::End => {
                sender.send(Zeo::End);
                return Ok(())
            },
            _ => return Err("bad method".into())
        }            
    }
}

fn writer(fs: Arc<filestorage::FileStorage>,
          mut stream: std::net::TcpStream,
          receiver: std::sync::mpsc::Receiver<Zeo>)
          -> Result<()> {

    for zeo in receiver.iter() {
        match zeo {
            Zeo::Error(id, name, message) => {
                try!(stream.write_all(&try!(encode!(
                    (id, "E", (name, (message,)))
                ))).chain_err(|| "stream write"));
            },
            Zeo::End => break,
            _ => {}
        }
    }
    Ok(())
}

fn main() {

    // To do, options :)
    let fs = Arc::new(
        filestorage::FileStorage::open(String::from("data.fs")).unwrap());
    
    let listener = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("Accepted {:?}", stream);
                let (sender, receiver) = std::sync::mpsc::channel();

                let read_fs = fs.clone();
                let read_stream = stream.try_clone().unwrap();
                std::thread::spawn(
                    move || reader(read_fs, read_stream, sender).unwrap());

                let write_fs = fs.clone();
                std::thread::spawn(
                    move || writer(write_fs, stream, receiver).unwrap());
            },
            Err(e) => { println!("WTF {}", e) }
        }
    }
}