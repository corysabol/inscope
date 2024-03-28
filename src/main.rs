use anyhow::Error;
use clap::{error, Parser, Subcommand};
use rusqlite::{Connection, Result};
use std::env;
use std::fs::read_to_string;
use std::io::{self, BufRead};
use std::net::{AddrParseError, IpAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::str::FromStr;
//use trust_dns_resolver::config::*;
//use trust_dns_resolver::Resolver;

const DEFAULT_PATH: &str = "./scope";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check IPs against the scope
    Check {
        /// IP address to check against scope DB
        #[arg(short, long)]
        ip: Option<String>,
        /// Path to the DB to check against
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Manipulate the database of IP addresses
    DB {
        /// The path to the database file
        #[arg(short, long)]
        path: Option<String>,
        /// An IP address to add to the scope
        #[arg(short, long)]
        ip: Option<String>,
        /// File containing a list of IP addresses to add to the scope
        #[arg(short, long)]
        list: Option<PathBuf>,
        /// Print out the IPs in the scope database to STDOUT
        #[arg(short, long)]
        show: bool,
    },
}

#[derive(Debug)]
struct ScopedIp {
    id: i32,
    ip: String,
}

fn create_db(path: PathBuf) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(&path)?;

    conn.execute(
        "CREATE TABLE scope (
            id  INTEGER PRIMARY KEY,
            ip  TEXT UNIQUE
        )",
        (),
    )?;

    Ok(conn)
}

fn main() -> Result<(), rusqlite::Error> {
    let args = Args::parse();

    // check for subcommands first
    match &args.command {
        Some(Commands::Check { ip, path }) => {
            // If we get an IP positiional we check it against the databse
            match ip {
                Some(ip) => {
                    let path = match path {
                        Some(p) => p.clone(),
                        None => DEFAULT_PATH.to_string(),
                    };

                    let conn = Connection::open(path)?;
                    if is_in_scope(&conn, &ip.to_string())? {
                        println!("{ip}");
                    }
                }
                None => {
                    let path = match path {
                        Some(p) => p.clone(),
                        None => DEFAULT_PATH.to_string(),
                    };

                    let conn = Connection::open(path)?;

                    // If we get no args we assume that we are checking STDIN against the database
                    let stdin = io::stdin();
                    for ip in stdin.lock().lines() {
                        match IpAddr::from_str(&ip.as_ref().unwrap()) {
                            Ok(_) => {
                                if is_in_scope(&conn, &ip.as_ref().unwrap())? {
                                    println!("{}", ip.unwrap());
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }

        Some(Commands::DB {
            path,
            ip,
            list,
            show,
        }) => {
            let path = match path {
                Some(p) => p.clone(),
                None => DEFAULT_PATH.to_string(),
            };
            let conn = create_db(path.into())?;

            // add the value of --ip to the scope if we have it
            match ip {
                Some(ip) => {
                    match IpAddr::from_str(ip) {
                        Ok(_) => {
                            let _ = add_to_scope(&conn, ip);
                        }
                        Err(_) => {
                            // skip
                            eprintln!("[-] Skipping: failed to parse IP - {ip:?}");
                        }
                    }
                }
                None => {}
            }

            // Init the DB with the items in the file passed to list
            // load the file of ips
            match list {
                Some(path_buf) => {
                    // Read the IPs from the provided file path
                    for line in read_to_string(path_buf).unwrap().lines() {
                        // Validate as IP
                        match IpAddr::from_str(line) {
                            Ok(_) => {
                                // store the IP
                                let _ = add_to_scope(&conn, line);
                            }
                            Err(_) => {
                                // Skip
                                eprintln!("[-] Skipping: failed to parse IP - {line:?}");
                            }
                        }
                    }
                }
                None => {
                    // try to read list of IPS from STDIN
                    let stdin = io::stdin();
                    for ip in stdin.lock().lines() {
                        match IpAddr::from_str(&ip.as_ref().unwrap()) {
                            Ok(_) => {
                                let _ = add_to_scope(&conn, &ip.as_ref().unwrap());
                            }
                            Err(_) => {
                                eprintln!("[-] Skipping: failed to parse IP - {ip:?}");
                            }
                        }
                    }
                }
            }
        }

        None => {}
    }

    // Setup DNS resolver
    //let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

    Ok(())
}

fn add_to_scope(conn: &Connection, ip: &str) -> Result<usize> {
    conn.execute("INSERT INTO scope (ip) VALUES (?1)", [ip])
}

fn is_in_scope(conn: &Connection, ip: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT ip FROM scope WHERE ip = ?1")?;
    let ip_iter = stmt.query_map([ip], |row| {
        Ok(ScopedIp {
            id: row.get(0)?,
            ip: row.get(1)?,
        })
    })?;

    for ip in ip_iter {
        if ip.is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}
