# Configuration
Configuration is basically a TOML file. Root sections describe
engine types (httpm, socks5, socks4, tcppm). Second level defines
names for engines. So by define multiple names user can start
multiple proxies.

The default configuration is:

```
[http.a]
port = 3128
```

This configuration used if no config file specified in arguments
and is basically a http proxy on port 3128.

Each engine has a set of options:

## tcppm

* port: port number to listen for incoming connections
* target: in _ip:port_ specifies the target to forward the
          connection

## socks4, socks5, http

* port: port number to listen for incoming connections

# Example

```
#define pair of proxies...
[http.one]
port = 8080
[http.other]
port = 3128
#...and one tcp forwarder
[tcppm.somename]
port = 65000
target = "127.0.0.1:3128"
```