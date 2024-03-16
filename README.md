# homebase

With homebase, you can turn a list of Regional Internet Registry (RIR) Organization IDs (Org IDs), Autonomous System (AS) Numbers, A and AAAA DNS records into IPv4 and IPv6 CIDR lists, for use in routers, firewalls, servers, and more.

Its intended use is to establish a "homebase" (of CIDRs) that are allowed access to particular resources.  It can also be used to block access from certain CIDRs.  It can even be used to replace or augment certain Dynamic DNS use cases.

## How to build

First, install rust and cargo via rustup.  Get it here:
https://rustup.rs/

Build homebase via cargo:

`cargo build -r`

The homebase binary will be placed at `target/release/homebase`.

To build via cargo and create a release package:

`./create_release.sh` (currently only written with Linux in mind)

## How to install

Homebase can be installed from the source repo or a release package by running:

`./install.sh`

## How to use

```
homebase v1.0.0
Copyright 2024, Matt Smith

Usage: ./homebase [-t, --txt HOST] [-s, --string STRING] [-c, --cache FILE] [-z46]

Options:
    -t, --txt HOST      specify homebase TXT host
    -s, --string STRING specify homebase string
    -c, --cache FILE    specify cache file
    -z, --sort          sort output
    -4, --ipv4          output IPv4 CIDRs only
    -6, --ipv6          output IPv6 CIDRs only
    -v, --version       print homebase version
    -h, --help          print this help menu
```

Use `-t` and/or `-s` to specify your homebase.

`-t, --txt HOST`: HOST is a TXT DNS host that specifies a homebase string.

`-s, --string STRING`: STRING specifies or appends to (if `-t` is also used) the homebase string.

The homebase string syntax is inspired by email-based SPF TXT DNS records and is used for both `-t` and `-s`:

- Values are space-separated
- Values can be:
  - `ORG:<RIR ORG ID>` (ie: ORG:ATT)
  - `AS:AS<RIR AS Number>` -or- `AS:<RIR AS Number>` (ie: AS:AS1 -or- AS:1)
  - `A:<Hostname>` (ie: A:google.com)
  - `AAAA:<Hostname>` (ie: AAAA:google.com)
  - `IPv4:<CIDR>` -or- `IPv4:<IP Address>` (ie: IPv4:1.1.1.1/24 -or- IPv4:1.1.1.1, which resolves to 1.1.1.1/32)
  - `IPv6:<CIDR>` -or- `IPv6:<IP Address>` (ie: IPv6:2001::1/64 -or- IPv6:2001::1, which resolves to 2001::1/128)

Search for ORG IDs and AS Numbers via [ARIN Whois](https://search.arin.net/rdap/) or [HE.net BGP Toolkit](https://bgp.he.net).

`-c, --cache FILE`: FILE specifies a path to a cache file.  The cache is invalidated after 1 hour or if the homebase string changes.

`-z, --sort`: Sort the CIDR output.

`-4, --ipv4`: Output IPv4 CIDRs only.

`-6, --ipv6`: Output IPv6 CIDRs only.

`-v, --version`: Print homebase version.

`-h, --help`: Print help menu.

## Examples

Example homebase TXT record:

```
$ dig TXT _homebase.example.com | awk '/"/ {gsub(/"/,"",$0); print substr($0,index($0,$5));}'
AS:3 IPv4:1.1.1.1
```

Example homebase commands:

Use the homebase TXT record `_homebase.example.com`, the homebase string `ORG:ATT AS:12`, and sort the output:
```
$ ./homebase -t _homebase.example.com -s "ORG:ATT AS:12" -z
```

Use the homebase string `AS:1`, output only IPv6 CIDRs, and sort the output:
```
$ ./homebase -s "AS:1" -6 -z
```

See the included `examples` directory for example scripts.
