use clap::Parser;
use env_logger;
use eyre::{Result, WrapErr};
use ipnetwork::Ipv4Network;
use colored::*;
use log::info;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Command-line interface
#[derive(Parser)]
#[command(name = "cidr", version, author, about = "Pretty-print CIDR info for one or more addresses")]
struct Cli {
    /// One or more IPs (with prefix), e.g. 10.10.10.1/16
    #[arg(value_name = "ADDRESS", num_args = 1..)]
    addresses: Vec<String>,

    /// Optional network mask (e.g. 255.255.248.0)
    #[arg(short = 'm', long = "mask", value_name = "MASK")]
    mask: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let specs = expand_args(&cli.addresses)?;

    for (i, spec) in specs.iter().enumerate() {
        let net = parse_network(spec, cli.mask.as_deref())?;
        info!("Parsed network: {}", net);
        print_network(&net);

        if i + 1 < specs.len() {
            println!();
        }
    }

    Ok(())
}

/// Expand a mix of full “IP/prefix” and “/prefix” args into all full specs,
/// defaulting the first-ever prefix-only to 192.168.1.1.
///
/// # Errors
/// Returns an error if any provided IP or prefix fails to parse.
fn expand_args(raw_args: &[String]) -> Result<Vec<String>> {
    // default base IP for any leading /prefix
    let default_ip = Ipv4Addr::new(192, 168, 1, 1);
    let mut last_ip: Option<Ipv4Addr> = Some(default_ip);
    let mut out = Vec::with_capacity(raw_args.len());

    for raw in raw_args {
        let spec = if raw.starts_with('/') {
            // strip leading slash
            let tok = &raw[1..];
            if tok.contains('/') {
                // full spec like "/192.168.1.1/20"
                let mut parts = tok.splitn(2, '/');
                let ip_str = parts.next().unwrap();
                last_ip = Some(Ipv4Addr::from_str(ip_str)?);
                tok.to_string()
            } else {
                // prefix-only like "/16"
                let ip = last_ip.unwrap();
                format!("{}/{}", ip, tok)
            }
        } else {
            // full spec like "10.10.10.1/21"
            let mut parts = raw.splitn(2, '/');
            let ip_str = parts.next().unwrap();
            last_ip = Some(Ipv4Addr::from_str(ip_str)?);
            raw.clone()
        };

        out.push(spec);
    }

    Ok(out)
}

/// Parse either “addr/prefix” or “addr” + separate netmask into an `Ipv4Network`
fn parse_network(address: &str, mask: Option<&str>) -> Result<Ipv4Network> {
    if let Some(mask_str) = mask {
        let ip = address
            .parse::<Ipv4Addr>()
            .wrap_err("Invalid IP address")?;
        let mask_ip = mask_str
            .parse::<Ipv4Addr>()
            .wrap_err("Invalid network mask")?;
        let mask_u32 = u32::from(mask_ip);
        let prefix = (32 - mask_u32.trailing_zeros()) as u8;
        Ipv4Network::new(ip, prefix).wrap_err("Failed to build network from mask")
    } else {
        Ipv4Network::from_str(address).wrap_err("Invalid address/prefix format")
    }
}

/// Pretty-print network info with right-justified, colored labels
/// (computed width), and “Total Addresses:” at the bottom.
fn print_network(net: &Ipv4Network) {
    let prefix = net.prefix();
    let netaddr = net.network();
    let bcast   = net.broadcast();
    let mask    = net.mask();
    let count: u64 = 1 << (32 - prefix);

    let mut labels = vec![
        "Network:",
        "Broadcast:",
        "Netmask:",
        "First Host:",
        "Last Host:",
        "Usable Addrs:",
    ];
    if count == 1 {
        labels = vec!["1 Address Total:"];
    }

    let label_width = labels
        .iter()
        .map(|s| s.len())
        .max()
        .expect("at least one label");

    let pad_label = |s: &str| format!("{:>width$}", s, width = label_width).yellow();

    println!(
        "{}",
        format!("{}/{}:", netaddr, prefix)
            .bold()
            .magenta()
    );

    println!(
        "  {}  {}  {:<16}",
        pad_label("Network:"),
        format!("0x{:08x}", u32::from(netaddr)).bright_black(),
        netaddr.to_string().cyan()
    );
    println!(
        "  {}  {}  {:<16}",
        pad_label("Broadcast:"),
        format!("0x{:08x}", u32::from(bcast)).bright_black(),
        bcast.to_string().cyan()
    );
    println!(
        "  {}  {}  {:<16}",
        pad_label("Netmask:"),
        format!("0x{:08x}", u32::from(mask)).bright_black(),
        mask.to_string().cyan()
    );

    if count == 1 {
        println!("  {}", pad_label("1 Address Total:"));
        return;
    }

    let usable = if count > 2 { count - 2 } else { 0 };
    if usable > 0 {
        let first = Ipv4Addr::from(u32::from(netaddr) + 1);
        let last  = Ipv4Addr::from(u32::from(bcast) - 1);
        println!(
            "  {}  {}  {:<16}",
            pad_label("First Host:"),
            format!("0x{:08x}", u32::from(first)).bright_black(),
            first.to_string().cyan()
        );
        println!(
            "  {}  {}  {:<16}",
            pad_label("Last Host:"),
            format!("0x{:08x}", u32::from(last)).bright_black(),
            last.to_string().cyan()
        );
    }

    println!(
        "  {}  {}",
        pad_label("Usable Addrs:"),
        format!("{}", usable).bright_red()
    );
}
