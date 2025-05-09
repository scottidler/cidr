use clap::Parser;
use eyre::{Result, WrapErr};
use ipnetwork::Ipv4Network;
use std::net::Ipv4Addr;
use std::str::FromStr;
use log::info;

/// Display CIDR network information for an IPv4 address
#[derive(Parser)]
#[command(name = "cidr", version, author)]
struct Cli {
    /// IP address with prefix (e.g., 10.10.10.1/21)
    address: String,

    /// Network mask (e.g., 255.255.248.0)
    mask: Option<String>,
}

/// Parse either “addr/prefix” or “addr” + separate netmask into an `Ipv4Network`
fn parse_network(address: &str, mask: Option<&str>) -> Result<Ipv4Network> {
    if let Some(mask_str) = mask {
        // Separate-netmask style: parse addr and mask, then compute prefix
        let ip = address
            .parse::<Ipv4Addr>()
            .wrap_err("Invalid IP address")?;
        let mask_ip = mask_str
            .parse::<Ipv4Addr>()
            .wrap_err("Invalid network mask")?;
        let mask_u32 = u32::from(mask_ip);
        // trailing_zeros() returns a u32; cast the computed prefix into u8
        let prefix = (32 - mask_u32.trailing_zeros()) as u8;
        Ipv4Network::new(ip, prefix).wrap_err("Failed to build network from mask")
    } else {
        // addr/prefix style
        Ipv4Network::from_str(address).wrap_err("Invalid address/prefix format")
    }
}

/// Pretty-print exactly the same fields and formatting as your Python version
fn print_network(net: &Ipv4Network) {
    let prefix = net.prefix();
    let netaddr = net.network();
    let bcast   = net.broadcast();
    // rename netmask() → mask()
    let mask    = net.mask();
    let count: u64 = 1 << (32 - prefix);

    // Header
    println!("{}/{}:", netaddr, prefix);

    // Single-address case
    if count == 1 {
        println!(
            "  address:              {:<16}0x{:08x}",
            netaddr,
            u32::from(netaddr)
        );
    } else {
        // Network + broadcast
        println!(
            "  network address:      {:<16}0x{:08x}",
            netaddr,
            u32::from(netaddr)
        );
        println!(
            "  broadcast address:    {:<16}0x{:08x}",
            bcast,
            u32::from(bcast)
        );
    }

    // Mask
    println!(
        "  network mask:         {:<16}0x{:08x}",
        mask,
        u32::from(mask)
    );

    // Totals
    if count == 1 {
        println!("  1 address total");
    } else {
        let usable = if count > 2 { count - 2 } else { 0 };
        println!("  {} addresses total ({} usable)", count, usable);

        // First/last hosts, if any
        if count > 2 {
            let first = Ipv4Addr::from(u32::from(netaddr) + 1);
            let last  = Ipv4Addr::from(u32::from(bcast) - 1);
            println!(
                "  first host address:   {:<16}0x{:08x}",
                first,
                u32::from(first)
            );
            println!(
                "  last host address:    {:<16}0x{:08x}",
                last,
                u32::from(last)
            );
        }
    }
}

fn main() -> Result<()> {
    // set up logging (in case you want to add debug/info logs later)
    env_logger::init();
    let cli = Cli::parse();

    let net = parse_network(&cli.address, cli.mask.as_deref())?;
    info!("Parsed network: {}", net);

    print_network(&net);
    Ok(())
}
