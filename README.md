# inscope 🌐

`inscope` is a Rust-based command-line interface (CLI) tool designed for penetration testers and security professionals. It allows for efficient management and verification of penetration testing scopes using local databases. With `inscope`, users can easily add, remove, and check IP addresses against a locally maintained scope database, streamlining the process of scope management during security assessments.

## Features 🛠️

- **Manage Scopes:** Add, remove, and list IP addresses or ranges in your pentest scope with ease.
- **Check IPs:** Quickly verify if an IP address is within your defined pentest scope.
- **Local Database:** Utilizes a local database for fast access and control over your pentest scopes.

## Installation 🚀
### From Prebuilt Binaries (recommended)
TODO: Configure build and release pipeline

### From Source
Ensure you have Rust installed on your system. To install `inscope`, clone the repository and build the project:

```sh
git clone https://github.com/yourusername/inscope.git
cd inscope
cargo install --path .
```

## Usage 🔍

```
Usage: inscope [COMMAND]

Commands:
  check  Check IPs against the scope
  db     Manipulate the database of IP addresses
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Interacting with scope databases
```
Manipulate the database of IP addresses

Usage: inscope db [OPTIONS]

Options:
  -p, --path <PATH>  The path to the database file
  -i, --ip <IP>      An IP address to add to the scope
  -l, --list <LIST>  File containing a list of IP addresses to add to the scope
  -s, --show         Print out the IPs in the scope database to STDOUT
  -h, --help         Print help
```

### Checking scope

```
Check IPs against the scope

Usage: inscope check [OPTIONS]

Options:
  -i, --ip <IP>      IP address to check against scope DB
  -p, --path <PATH>  Path to the DB to check against
  -h, --help         Print help
```

## Contributing 🤝

Contributions are welcome! Feel free to submit a pull request or open an issue for any features, bugs, or improvements.

## License 📄

`inscope` is released under the MIT License. See the LICENSE file for more details.
