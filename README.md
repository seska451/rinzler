```
         _             __         
   _____(_)___  ____  / /__  _____
  / ___/ / __ \/_  / / / _ \/ ___/
 / /  / / / / / / /_/ /  __/ /    
/_/  /_/_/ /_/ /___/_/\___/_/     
                                  
🙌       a fast webcrawler        🙌
🙌       from seska with ♡♡♡      🙌
```
# Features
- Webcrawler, hunts for links and follows them
- Can be given a list of hosts to crawl
- By default, is limited to browsing within its original scope
- Can be unshackled to exhaust all URLs
- Supports startup options via env vars

## Planned
- forced browsing via wordlist
- simple fuzzing

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
    rinzler [OPTIONS] <HOST URL>

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
rinzler --help
```
## crawling a single host
```bash
rinzler https://crawler-test.com 
```
## crawling multiple hosts
```bash
rinzler --host https://crawler-test.com --host https://seska.io 
```
## rate limiting requests (50ms per request)
```bash
rinzler --host https://crawler-test.com --rate-limit 50
```
## run an unscoped crawl
```bash
rinzler --host https://crawler-test.com --scoped=false 
```
## customize the UA header
```bash
rinzler --host https://crawler-test.com --user-agent="Mozilla/5.0 (Linux; Android 8.0.0; SM-G960F Build/R16NW) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/62.0.3202.84 Mobile Safari/537.36" 
```
## suppress the banner on start
```bash
rinzler --host https://crawler-test.com --quiet 
```
