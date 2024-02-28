extern crate bitcoin;

// use core::slice::SlicePattern;
use std::io::{BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, process};

use bitcoin::consensus::{encode, Decodable};
use bitcoin::p2p::{self, address, message, message_network};
use bitcoin::secp256k1::rand::Rng;

/**
 * To run as a server:
 * cargo run --example handshake --features="std rand-std" S 127.0.0.1:8333
 *
 * To run as a client:
 * cargo run --example handshake --features="std rand-std" C 127.0.0.1:8333
 **/
fn main() {
    // This example establishes a connection to a Bitcoin node, sends the initial
    // "version" message, waits for the reply, and finally closes the connection.
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("not enough arguments");
        print_help();
        process::exit(1);
    }

    let (mut stream, address): (TcpStream, SocketAddr) = match args[1].as_str() {
        "C" => connect(args).unwrap(),
        "S" => accept(args).unwrap(),
        _ => {
            eprintln!("Invalid option");
            print_help();
            process::exit(1);
        }
    };

    let version_message = build_version_message(address);

    let first_message =
        message::RawNetworkMessage::new(bitcoin::Network::Bitcoin.magic(), version_message);

    // Send the message
    let _ = stream.write_all(encode::serialize(&first_message).as_slice());
    println!("Sent version message");

    // Setup StreamReader
    let read_stream = stream.try_clone().unwrap();
    let mut stream_reader = BufReader::new(read_stream);
    loop {
        // Loop an retrieve new messages
        let reply = message::RawNetworkMessage::consensus_decode(&mut stream_reader).unwrap();
        match reply.payload() {
            message::NetworkMessage::Version(_) => {
                println!("Received version message: {:?}", reply.payload());

                let second_message = message::RawNetworkMessage::new(
                    bitcoin::Network::Bitcoin.magic(),
                    message::NetworkMessage::Verack,
                );

                let _ = stream.write_all(encode::serialize(&second_message).as_slice());
                println!("Sent verack message");
            }
            message::NetworkMessage::Verack => {
                println!("Received verack message: {:?}", reply.payload());
                break;
            }
            _ => {
                println!("Received unknown message: {:?}", reply.payload());
                break;
            }
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

fn accept(args: Vec<String>) -> Result<(TcpStream, SocketAddr), std::io::Error> {
    let str_address = &args[2];

    let address: SocketAddr = str_address.parse().unwrap_or_else(|error| {
        eprintln!("Error parsing address: {:?}", error);
        process::exit(1);
    });

    let listener = TcpListener::bind(address).unwrap();
    listener.accept()
}

fn connect(args: Vec<String>) -> Result<(TcpStream, SocketAddr), std::io::Error> {
    let str_address = &args[2];

    let address: SocketAddr = str_address.parse().unwrap_or_else(|error| {
        eprintln!("Error parsing address: {:?}", error);
        process::exit(1);
    });

    let stream = TcpStream::connect(address)?;
    Ok((stream, address))
}

fn print_help() -> () {
    println!("first arg: S - act as server, C - act as client");
    println!("second arg: socket address format (ip:port)")
}

fn build_version_message(address: SocketAddr) -> message::NetworkMessage {
    // Building version message, see https://en.bitcoin.it/wiki/Protocol_documentation#version
    let my_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

    // "bitfield of features to be enabled for this connection"
    let services = p2p::ServiceFlags::NONE;

    // "standard UNIX timestamp in seconds"
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time error").as_secs();

    // "The network address of the node receiving this message"
    let addr_recv = address::Address::new(&address, p2p::ServiceFlags::NONE);

    // "The network address of the node emitting this message"
    let addr_from = address::Address::new(&my_address, p2p::ServiceFlags::NONE);

    // "Node random nonce, randomly generated every time a version packet is sent. This nonce is used to detect connections to self."
    let nonce: u64 = secp256k1::rand::thread_rng().gen();

    // "User Agent (0x00 if string is 0 bytes long)"
    let user_agent = String::from("rust-example");

    // "The last block received by the emitting node"
    let start_height: i32 = 0;

    // Construct the message
    message::NetworkMessage::Version(message_network::VersionMessage::new(
        services,
        timestamp as i64,
        addr_recv,
        addr_from,
        nonce,
        user_agent,
        start_height,
    ))
}
