```
         _             __         
   _____(_)___  ____  / /__  _____
  / ___/ / __ \/_  / / / _ \/ ___/
 / /  / / / / / / /_/ /  __/ /    
/_/  /_/_/ /_/ /___/_/\___/_/     
                                  
🙌       a fast webcrawler        🙌
🙌       from seska with ♡♡♡      🙌
```
[![asciicast](https://asciinema.org/a/dEqkvxT2dtt9jFQdXw1tmvGwZ.svg)](https://asciinema.org/a/dEqkvxT2dtt9jFQdXw1tmvGwZ)
# Features
- Webcrawler
- Fuzzer
- Force Browser
- Multihost
- Scoped, or unscoped crawling
- Easy to use
- Can be configured with environment variables

## Planned
You can see what we're planning for v1.0 here https://github.com/seska451/rinzler/milestone/1

# Installation

Requires rust to compile from source

By default, it will install to `$HOME/bin` so make sure that is on your `$PATH`!
```bash
git clone https://github.com/seska451/rinzler.git
cd rinzler
make install
```

# Usage by example
```bash
USAGE:
    rz [OPTIONS] <HOST URL>

ARGS:
    <HOST URL>    The host URL to scan

OPTIONS:
    -h, --host <HOST URL>            Set the initial URL to start crawling. Can be set multiple
                                     times to crawl several sites at once. [env: RINZLER_HOSTS=]
        --help                       Print help information
    -q, --quiet                      When set, this flag suppresses extraneous output like the
                                     version banner.
    -r, --rate-limit <rate-limit>    Set the number of milliseconds to wait between each request.
                                     [env: RINZLER_RATELIMIT=] [default: 0]
    -s, --scoped <scoped>            Prevents rinzler from searching beyond the original domains
                                     specified. Defaults to true. [default: true]
    -u, --user-agent <user-agent>    Set the user-agent header. Defaults to '0.0.1-alpha' [env:
                                     RINZLER_UA=] [default: "rinzler v0.0.1-alpha"]
    -v                               Sets the level of output verbosity. Set multiple times 
    -V, --version                    Print version information

```
## get help
```bash
rz --help
```
## crawling a single host
```bash
rz https://crawler-test.com 
```
## crawling multiple hosts
```bash
rz --host https://crawler-test.com --host https://seska.io 
```
## rate limiting requests (50ms per request)
```bash
rz --host https://crawler-test.com --rate-limit 50
```
## run an unscoped crawl
```bash
rz --host https://crawler-test.com --scoped=false 
```
## customize the UA header
```bash
rz --host https://crawler-test.com --user-agent="Mozilla/5.0 (Linux; Android 8.0.0; SM-G960F Build/R16NW) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/62.0.3202.84 Mobile Safari/537.36" 
```
## suppress the banner on start
```bash
rz --host https://crawler-test.com --quiet 
```
