# profanity2-rs

# ⚠️Experimental Project⚠️
profanity2-rs is a Rust port version of the original [profanity2](https://github.com/1inch/profanity2). It's just a product of me starting to write some Rust code and trying to learn the language. It's not meant to be used in production, but DYOR if you want to use it.

Profanity is a high performance (probably the fastest!) vanity address generator for Ethereum. Create cool customized addresses that you never realized you needed! Recieve Ether in style! Wow!

![Screenshot](/screenshot.png?raw=true "Wow! That's a lot of zeros!")

# Important to know

A previous version of this project has a known critical issue due to a bad source of randomness. The issue enables attackers to recover private key from public key: https://blog.1inch.io/a-vulnerability-disclosed-in-profanity-an-ethereum-vanity-address-tool-68ed7455fc8c

This project "profanity2" was forked from the original project and modified to guarantee **safety by design**. This means source code of this project do not require any audits, but still guarantee safe usage.

Project "profanity2" is not generating key anymore, instead it adjusts user-provided public key until desired vanity address will be discovered. Users provide seed public key in form of 128-symbol hex string with `-seed` parameter flag. Resulting private key should be used to be added to seed private key to achieve final private key of the desired vanity address (private keys are just 256-bit numbers). Running "profanity2" can even be outsourced to someone completely unreliable - it is still safe by design.

## Getting public key for mandatory `-seed` parameter

Generate private key and public key via openssl in terminal (remove prefix "04" from public key):
```bash
$ openssl ecparam -genkey -name secp256k1 -text -noout -outform DER | xxd -p -c 1000 | sed 's/41534e31204f49443a20736563703235366b310a30740201010420/Private Key: /' | sed 's/a00706052b8104000aa144034200/\'$'\nPublic Key: /'
```

Derive public key from existing private key via openssl in terminal (remove prefix "04" from public key):
```bash
$ openssl ec -inform DER -text -noout -in <(cat <(echo -n "302e0201010420") <(echo -n "PRIVATE_KEY_HEX") <(echo -n "a00706052b8104000a") | xxd -r -p) 2>/dev/null | tail -6 | head -5 | sed 's/[ :]//g' | tr -d '\n' && echo
```

## Adding private keys (never use online calculators!)

### Terminal:

Use private keys as 64-symbol hexadecimal string WITHOUT `0x` prefix:
```bash
(echo 'ibase=16;obase=10' && (echo '(PRIVATE_KEY_A + PRIVATE_KEY_B) % FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F' | tr '[:lower:]' '[:upper:]')) | bc
```

### Python

Use private keys as 64-symbol hexadecimal string WITH `0x` prefix:
```bash
$ python3
>>> hex((PRIVATE_KEY_A + PRIVATE_KEY_B) % 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F)
```

# Usage
```
Usage: profanity2-rs [OPTIONS] --seed <SEED> <COMMAND>

Commands:
  benchmark      Run without any scoring, a benchmark.
  doubles        Score on hashes leading with hexadecimal pairs.
  leading        Score on hashes leading with given hex character.
  leading-range  Score on hashes leading with characters within given range.
  letters        Score on letters anywhere in hash.
  matching       Score on hashes matching given hex string.
  mirror         Score on mirroring from center.
  numbers        Score on numbers anywhere in hash.
  range          Score on hashes having characters within given range anywhere.
  zeros          Score on zeros anywhere in hash.
  help           Print this message or the help of the given subcommand(s)

Options:
  -s, --seed <SEED>
          Set seed to use for address generation.
      --skip-devices <SKIP_DEVICES>
          Skip devices with given indices (comma separated).
  -W, --work-max <MAX_WORK_SIZE>
          Set OpenCL maximum work size. Default to [-i * -I]. [default: 4177920]
  -w, --work <WORK_SIZE>
          Set OpenCL local work size. [default: 64]
  -i, --inverse-size <INVERSE_SIZE>
          Set size of modular inverses to calculate in one work item. [default: 255]
  -I, --inverse-multiplier <INVERSE_MULTIPLIER>
          Set how many above work items will run in parallell. [default: 16384]
      --compact-speed
          Only show total iteration speed.
  -t, --target <TARGET>
          Set target to search for [default: address] [possible values: address, contract]
  -h, --help
          Print help
  -V, --version
          Print version
```

### Benchmarks - Current version [from profanity]
|Model|Clock Speed|Memory Speed|Modified straps|Speed|Time to match eight characters
|:-:|:-:|:-:|:-:|:-:|:-:|
|GTX 1070 OC|1950|4450|NO|179.0 MH/s| ~24s
|GTX 1070|1750|4000|NO|163.0 MH/s| ~26s
|RX 480|1328|2000|YES|120.0 MH/s| ~36s
|Apple Silicon M1<br/>(8-core GPU)|-|-|-|45.0 MH/s| ~97s
|Apple Silicon M1 Max<br/>(32-core GPU)|-|-|-|172.0 MH/s| ~25s