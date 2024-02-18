use std::str::FromStr;
use async_recursion::async_recursion;

use rustdns::Message;
use rustdns::types::*;
use std::net::UdpSocket;
use std::time::{Duration, SystemTime};

use ipnetwork::IpNetwork;

use getopts::Options;
use std::env;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use std::fs;
use std::io::Write;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;

#[async_recursion]
async fn rdap_query(s: String) -> Result<String, String> {
    let rdap_config = icann_rdap_client::ClientConfig::default();
    let rdap_client = icann_rdap_client::create_client(&rdap_config).map_err(|e| format!("{}", e.to_string()))?;
    let rdap_query_string = &s.replace("AS:", "").replace("ORG:", "");
    let rdap_query_ = icann_rdap_client::QueryType::from_str(&rdap_query_string).map_err(|e| format!("{}", e.to_string()))?;
    let rdap_base_url = "https://rdap-bootstrap.arin.net/bootstrap";
    let rdap_response = icann_rdap_client::rdap_request(rdap_base_url, &rdap_query_, &rdap_client).await.map_err(|e| format!("RDAP Query Error: Query: \"{}\": {}", &s, &e))?;
    let rdap_response_json: serde_json::Value = serde_json::to_value(&rdap_response.rdap).unwrap();
    let mut output = String::new();
    
    if s.contains("AS:") {
        let as_org = rdap_response_json["entities"][0]["handle"].to_string().replace('"', "");
        match rdap_query(format!("ORG:{}", as_org)).await {
            Ok(o) => { output = o; },
            Err(e) => { if e.contains(": No CIDRs returned") { return Err(format!("RDAP Query Warning: Query \"{}\": No CIDRs returned", &s)); } else { return Err(e); } },
        }
    } else {
        if rdap_response_json.as_object().unwrap().contains_key("networks") {
            let networks = rdap_response_json["networks"].as_array().unwrap();
            for network in networks {
                let cidr = &network["cidr0_cidrs"][0];
                if cidr["v6prefix"].is_string() {
                    output = format!("{}{}/{}\n", output, cidr["v6prefix"].to_string().replace('"', "").to_lowercase(), cidr["length"]);
                } else if cidr["v4prefix"].is_string() {
                    output = format!("{}{}/{}\n", output, cidr["v4prefix"].to_string().replace('"', ""), cidr["length"]);
                }
            }
            output = output.trim().to_string();
        }
    }

    if output.len() > 0 {
        Ok(output)
    } else {
        Err(format!("RDAP Query Warning: Query: \"{}\": No CIDRs returned", &s))
    }
}

fn dns_query(nameserver: &String, query_hostname: String, query_type: rustdns::Type) -> Result<Vec<Record>, String> {
    let nameserver_and_port = format!("{}:53", nameserver);
    let mut m = Message::default();
    m.add_question(&query_hostname, query_type, Class::Internet);
    m.add_extension(Extension {
        payload_size: 4096,
        ..Default::default()
    });

    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("DNS Bind Error: {}", e.to_string()))?;
    socket.set_read_timeout(Some(Duration::new(5, 0))).map_err(|e| format!("DNS Set Read Timeout Error: {}", e.to_string()))?;
    socket.connect(nameserver_and_port).map_err(|e| format!("DNS Connect Error: {}", e.to_string()))?;

    let question = m.to_vec().map_err(|e| format!("DNS Error Converting Message to Vector: {}", e.to_string()))?;

    socket.send(&question).map_err(|e| format!("DNS Error Sending Query: {}", e.to_string()))?;

    let mut resp = [0; 4096];
    let len = socket.recv(&mut resp).map_err(|e| format!("DNS Error Receiving Response: {}", e.to_string()))?;

    let answer = Message::from_slice(&resp[0..len]).map_err(|e| format!("DNS Error Getting Message From Slice: {}", e.to_string()))?;

    Ok(answer.answers)
}

fn get_system_nameserver() -> String {
    let mut nameserver = String::new();
    let file = std::fs::read_to_string("/etc/resolv.conf").expect("Error reading file: /etc/resolv.conf");
    let lines = file.lines();
    for line in lines {
        if line.chars().nth(0) == Some('#') || line == "" {
            continue;
        }
        if line.split(" ").nth(0) == Some("nameserver") {
            nameserver = line.split(" ").nth(1).unwrap().to_string();
            break;
        }
    }

    nameserver
}

async fn process_string(string: &String) -> String {
    let nameserver = get_system_nameserver();
    let mut output = String::new();
    let mut rdap_result: Result<String, String>;

    for piece in string.trim().split(" ") {
        if piece.split(":").count() >= 2 {
            let key = piece.split(":").nth(0).unwrap().to_uppercase();
            let val = piece.split(":").nth(1).unwrap().to_uppercase();
            if key == "ORG" {
                rdap_result = rdap_query(format!("ORG:{}", val)).await;
                match rdap_result {
                    Ok(o) => { output = format!("{}{}\n", output, o); },
                    Err(e) => { eprintln!("{}", e); },
                }
            }
            else if key == "AS" {
                if val.contains("AS") {
                    rdap_result = rdap_query(format!("AS:{}", val)).await;
                } else {
                    rdap_result = rdap_query(format!("AS:AS{}", val)).await;
                }
                match rdap_result {
                    Ok(o) => { output = format!("{}{}\n", output, o); },
                    Err(e) => { eprintln!("{}", e); },
                }
            }
            else if key == "A" {
                let answer = dns_query(&nameserver, val.to_string(), Type::A);
                match answer {
                    Ok(o) => {
                        if o.len() > 0 {
                            for a in o {
                                output = format!("{}{}/32\n", output, a.resource.to_string().replace('"', ""));
                            }
                        } else {
                            eprintln!("DNS Query Warning: Query: \"A:{}\" Warning: No DNS results", &val);
                        }
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                    },
                }
            }
            else if key == "AAAA" {
                let answer = dns_query(&nameserver, val.to_string(), Type::AAAA);
                match answer {
                    Ok(o) => {
                        if o.len() > 0 {
                            for a in o {
                                output = format!("{}{}/128\n", output, a.resource.to_string().replace('"', ""));
                            }
                        } else {
                            eprintln!("DNS Query Warning: Query \"AAAA:{}\" Warning: No DNS results", &val);
                        }
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                    },
                }
            }
            else if key == "IPV4" {
                if val.contains("/") {
                    output = format!("{}{}\n", output, val);
                } else {
                    output = format!("{}{}/32\n", output, val);
                }
            }
            else if key == "IPV6" {
                let val: String = piece.to_lowercase().replace("ipv6:", "");
                if val.contains("/") {
                    output = format!("{}{}\n", output, val);
                } else {
                    output = format!("{}{}/128\n", output, val);
                }
            }
        }
    }

   output
}

#[derive(Serialize, Deserialize)]
struct HomebaseCache {
    produced_at: SystemTime,
    homebase_string: String,
    cidrs: Vec<IpNetwork>,
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [-t, --txt HOST] [-s, --string STRING] [-c, --cache FILE] [-z46]", program);
    print!("{}", opts.usage(&brief));
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();

    let mut homebase_string = String::new();
    let mut cidrs: Vec<IpNetwork> = vec![];

    let mut use_cache: bool = false;
    let mut cache_file_path = String::new();
    let mut cache = HomebaseCache {
        produced_at: SystemTime::now(),
        homebase_string: String::new(),
        cidrs: vec![],
    };

    // configure and parse options
    opts.optopt("t", "txt",     "specify homebase TXT host", "HOST");
    opts.optopt("s", "string",  "specify homebase string", "STRING");
    opts.optopt("c", "cache",   "specify cache file", "FILE");
    opts.optflag("z", "sort",   "sort output");
    opts.optflag("4", "ipv4",   "output IPv4 CIDRs only");
    opts.optflag("6", "ipv6",   "output IPv6 CIDRs only");
    opts.optflag("h", "help",   "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(_f) => { print_usage(&program, opts); std::process::exit(1); }
    };

    // print help menu
    if matches.opt_present("h") || args.len() == 1 ||
        (! matches.opt_present("t") && ! matches.opt_present("s")) ||
        (matches.opt_present("t") && matches.opt_str("t").unwrap().len() == 0) ||
        (matches.opt_present("s") && matches.opt_str("s").unwrap().len() == 0) ||
        (matches.opt_present("c") && matches.opt_str("c").unwrap().len() == 0) {
        print_usage(&program, opts);
        std::process::exit(1);
    }

    // build homebase string from TXT record
    if matches.opt_present("t") {
        let nameserver = get_system_nameserver();
        let homebase_query = dns_query(&nameserver, matches.opt_str("t").unwrap(), Type::TXT);
        match homebase_query {
            Ok(o) => { if o.len() > 0 { homebase_string = o[0].resource.to_string().replace('"', ""); } },
            Err(e) => { eprintln!("{}", e.to_string()); },
        }
    }

    // set/append to homebase string
    if matches.opt_present("s") {
        homebase_string = format!("{} {}", homebase_string, matches.opt_str("s").unwrap());
    }

    // trim leading/trailing whitespace from homebase string
    homebase_string = homebase_string.trim().to_string();

    // if cache file is specified, read it and determine if we should use it
    if matches.opt_present("c") {
        cache_file_path = matches.opt_str("c").unwrap();
        if Path::new(&cache_file_path).exists() {
            // get the cache JSON from the cache file
            let cache_json_string = std::fs::read_to_string(&cache_file_path).expect("Error reading homebase cache file");
            // populate the homebase cache object from the cache JSON
            cache = serde_json::from_str(cache_json_string.as_str()).unwrap();
            // determine if we should use the cache or start fresh
            let now = SystemTime::now();
            if now.duration_since(cache.produced_at).unwrap().as_secs() >= 3600 || homebase_string != cache.homebase_string {
                use_cache = false;
            } else {
                use_cache = true;
            }
        }
    }

    // start fresh, don't use cache
    if ! use_cache {
        // process homebase string and return CIDRs as string
        let mut cidrs_string = process_string(&homebase_string).await;
        cidrs_string = cidrs_string.trim().to_string();
        let mut seen = HashMap::new();

        if cidrs_string.len() == 0 {
            eprintln!("Homebase Warning: No CIDRs returned");
            std::process::exit(1);
        }

        // fill the cidrs vector with the CIDRs from cidrs_string
        for cidr_string in cidrs_string.lines() {
            let mut cidr_vec = vec![cidr_string.parse().unwrap()];

            // add CIDR to the cidrs vector if we haven't seen it before (prevent duplicates)
            if ! seen.contains_key(&cidr_string) {
                // add both IPv4 and IPv6 CIDRs to cidrs vector
                if (! matches.opt_present("4") && ! matches.opt_present("6")) || (matches.opt_present("4") && matches.opt_present("6")) {
                    cidrs.append(&mut cidr_vec);
                }
                // add only IPv4 CIDRs to cidrs vector
                else if matches.opt_present("4") && cidr_string.contains(".") {
                    cidrs.append(&mut cidr_vec);
                }
                // add only IPv6 CIDRs to cidrs vector
                else if matches.opt_present("6") && cidr_string.contains(":") {
                    cidrs.append(&mut cidr_vec);
                }

                // take note that we have now seen this CIDR
                seen.insert(cidr_string, true);
            }
        }

        // sort CIDRs, if specified
        if matches.opt_present("z") {
            cidrs.sort();
        }

        // populate the homebase cache object
        cache = HomebaseCache {
            produced_at: SystemTime::now(),
            homebase_string: homebase_string.clone(),
            cidrs: cidrs,
        };

        // write out JSON cache file, if specified
        if matches.opt_present("c") {
            let cache_json = serde_json::to_string(&cache).unwrap();
            let mut cache_file = fs::File::create(&cache_file_path).unwrap();
            fs::set_permissions(&cache_file_path, fs::Permissions::from_mode(0o600)).unwrap();
            write!(cache_file, "{}", cache_json).expect("Failed to write cache file");
        }
    }

    if cache.cidrs.len() == 0 {
        eprintln!("Homebase Warning: No CIDRs returned");
        std::process::exit(1);
    }

    // print out list of deduplicated, filtered, and sorted CIDRs
    for cidr in cache.cidrs {
        println!("{}", cidr);
    }
    std::process::exit(0);
}
