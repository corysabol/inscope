use clap::{error, Parser, Subcommand};
use rusqlite::{Connection, Result};
use std::env;
use std::fmt;
use std::fs::read_to_string;
use std::io::{self, BufRead};
use std::net::{AddrParseError, IpAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::str::FromStr;
//use trust_dns_resolver::config::*;
//use trust_dns_resolver::Resolver;

const DEFAULT_PATH: &str = "./scope.sqlite3";

#[derive(Parser)]
#[command(version="0.2.0", about, long_about = None)]
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
    /// Print the scope to STDOUT
    Show {
        /// Optional - The path to the database file
        #[arg(short, long)]
        path: Option<String>,

        /// Show only IPs
        #[arg(short, long)]
        ip: bool,

        /// Show only hosts with names
        #[arg(short, long)]
        names: bool,

        /// Show live hosts
        #[arg(short, long)]
        live: bool,
    },
    /// Add to the scope of IP addresses
    /// IP addresses will be expected from STDIN in newline delimited format if no flags are given
    /// Each IP address may also have a list of comma delimited hostnames to associate them
    /// E.g.
    ///
    /// 138.155.2.5
    /// 192.168.1.1,foo.bar.com,bar.com
    /// 198.111.11.1,example.net,anotherexample.com,foo.bar.net
    Add {
        /// The path to the database file
        #[arg(short, long)]
        path: Option<String>,

        /// An IP address to add to the scope, can have an associated list of names
        /// -i 192.168.1.1,foo.bar.com,bar.com
        #[arg(short, long)]
        ip: Option<String>,

        /// File containing a list of IP addresses to add to the scope
        /// Each line can have an associated list of names that are mapped to the IP
        /// 138.155.2.5
        /// 192.168.1.1,foo.bar.com,bar.com
        /// 198.111.11.1,example.net,anotherexample.com,foo.bar.net
        #[arg(short, long)]
        list: Option<PathBuf>,

        /// Mark the given host or list of hosts as being live hosts
        #[arg(long)]
        live: bool,
    },
    /// Remove items from the scope !!NOT YET IMPLEMENTED!!
    Remove {},
}

#[derive(Debug)]
struct ScopedIp {
    id: i32,
    ip: String,
    names: String,
    live: bool,
}

impl fmt::Display for ScopedIp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.names == "" {
            write!(f, "{}", self.ip)
        } else {
            write!(f, "{},{}", self.ip, self.names.clone())
        }
    }
}

fn create_db(path: PathBuf) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(&path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS scope (
            id  INTEGER PRIMARY KEY,
            ip  TEXT UNIQUE,
            names TEXT,
            live INTEGER
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

        Some(Commands::Show {
            path,
            ip,
            names,
            live,
        }) => {
            let path = match path {
                Some(p) => p.clone(),
                None => DEFAULT_PATH.to_string(),
            };
            let conn = create_db(path.into())?;

            // Print the scope to stdout
            let scope_result = get_scope(&conn);
            match scope_result {
                Ok(scope) => {
                    for ip in scope {
                        //println!("{}", ip.ip);
                        println!("{}", ip);
                    }
                }
                Err(err) => {
                    eprintln!("Couldn't get scope list due to error {:?}", err)
                }
            }
        }

        Some(Commands::Add {
            path,
            ip,
            list,
            live,
        }) => {
            let path = match path {
                Some(p) => p.clone(),
                None => DEFAULT_PATH.to_string(),
            };
            let conn = create_db(path.into())?;
            if ip.is_none() && list.is_none() {
                // try to read list of IPS from STDIN
                let stdin = io::stdin();
                for ip in stdin.lock().lines() {
                    let parts: Vec<&str> = ip.as_ref().unwrap().split(",").collect();
                    let ip_str = parts[0];
                    match IpAddr::from_str(ip_str) {
                        Ok(_) => {
                            let _ = add_to_scope(&conn, &ip.as_ref().unwrap(), *live);
                        }
                        Err(_) => {
                            eprintln!("[-] Skipping: failed to parse IP - {ip:?}");
                        }
                    }
                }
            } else {
                // add the value of --ip to the scope if we have it
                match ip {
                    Some(ip) => {
                        let parts: Vec<&str> = ip.split(",").collect();
                        let ip_str = parts[0];
                        match IpAddr::from_str(ip_str) {
                            Ok(_) => {
                                let _ = add_to_scope(&conn, ip, *live);
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
                            let parts: Vec<&str> = line.split(",").collect();
                            let ip_str = parts[0];
                            // Validate as IP
                            match IpAddr::from_str(ip_str) {
                                Ok(_) => {
                                    // store the IP
                                    let _ = add_to_scope(&conn, line, *live);
                                }
                                Err(_) => {
                                    // Skip
                                    eprintln!("[-] Skipping: failed to parse IP - {line:?}");
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
        }

        // TODO: Implement me
        Some(Commands::Remove {}) => {
            todo!("Implement remove command")
        }

        None => {}
    }

    // Setup DNS resolver
    //let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();

    Ok(())
}

fn add_to_scope(conn: &Connection, ip_str: &str, live: bool) -> Result<usize> {
    let is_live: u8 = if live { 1 } else { 0 };
    // check if we have any associated names
    let parts: Vec<String> = ip_str.split(",").map(str::to_string).collect();
    let ip = parts[0].clone();
    let names = if parts.len() > 1 {
        let p = &parts[1..parts.len()];
        p.join(",")
    } else {
        "".to_string()
    };
    conn.execute(
        "INSERT INTO scope (ip, names, live) VALUES (?1, ?2, ?3)",
        [ip, names.to_string(), is_live.to_string()],
    )
}

fn get_scope(conn: &Connection) -> Result<Vec<ScopedIp>> {
    let mut stmt = conn.prepare("SELECT id, ip, names, live FROM scope")?;

    let ip_iter = stmt.query_map([], |row| {
        let live_val: u8 = row.get(3)?;
        Ok(ScopedIp {
            id: row.get(0)?,
            ip: row.get(1)?,
            names: row.get(2)?,
            live: if live_val >= 1 { true } else { false },
        })
    })?;

    let mut ips = Vec::new();
    for ip_result in ip_iter {
        ips.push(ip_result?);
    }
    Ok(ips)
}

fn is_in_scope(conn: &Connection, ip: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT id, ip, names FROM scope WHERE ip = ?1")?;
    let ip_iter = stmt.query_map([ip], |row| {
        let live_val: u8 = row.get(3)?;
        Ok(ScopedIp {
            id: row.get(0)?,
            ip: row.get(1)?,
            names: row.get(2)?,
            live: if live_val >= 1 { true } else { false },
        })
    })?;

    for ip in ip_iter {
        if ip.is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}
