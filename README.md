# simple_ssh

**`simple_ssh`** is a lightweight, asynchronous Rust library that simplifies SSH operations such as executing remote commands, transferring files via SCP, and interactive PTY sessions. Built on top of the [`russh`](https://github.com/Eugeny/russh) and [`russh-keys`](https://github.com/Eugeny/russh) crates, it offers a streamlined API for secure and efficient SSH interactions.

## Features

- Asynchronous SSH client operations using `tokio`
- Execute remote shell commands with ease
- Transfer files securely using the SCP protocol
- Interactive PTY shell sessions
- IPv6 link-local address support with scope ID
- Public key and password authentication
- Minimalistic and focused API design

## Installation

Add `simple_ssh` to your project's `Cargo.toml`

```toml
[dependencies]
simple_ssh = { git = "https://github.com/juliankahlert/simple_ssh", tag = "v0.1.1" }
```

## Usage

### Executing Remote Commands

```rust
use simple_ssh::Session;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::init()
        .with_port(22)
        .with_host("192.168.0.100")
        .with_user("root")
        .with_passwd("toor")
        .build()?;

    let mut ssh = session.connect().await?;
    println!("Connected");

    let code = ssh.cmd("ls -la").await?;
    println!("Exitcode: {:?}", code);

    ssh.close().await?;
    Ok(())
}
```

### Transferring Files via SCP

```rust
use simple_ssh::Session;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::init()
        .with_port(22)
        .with_host("192.168.0.100")
        .with_user("root")
        .with_passwd("toor")
        .build()?;

    let mut ssh = session.connect().await?;
    println!("Connected");

    ssh.scp("local_file.txt", "/remote/path/remote_file.txt").await?;
    println!("File transferred successfully.");
    Ok(())
}
```

### IPv6 Link-Local Addresses

For IPv6 link-local addresses, use the `--scope` option to specify the network interface:

```rust
use simple_ssh::Session;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::init()
        .with_host("fe80::1")
        .with_scope("eth0")  // Network interface name or index
        .with_user("root")
        .with_passwd("password")
        .build()?;

    let mut ssh = session.connect().await?;
    // ...
    Ok(())
}
```

## CLI Tool

A statically compiled `simple-ssh` binary is included for direct command-line use:

```bash
# Interactive shell
simple-ssh -H 192.168.1.1 -u root -P password

# Execute command
simple-ssh -H server.example.com -u admin -P secret "uname -a"

# With private key
simple-ssh -H server.example.com -i /path/to/key

# IPv6 link-local with scope
simple-ssh -H fe80::1%eth0 -u root -P password
```

### CLI Options

| Option | Description |
|--------|-------------|
| `-H, --host <HOST>` | SSH host to connect to (required) |
| `-u, --user <USER>` | SSH username (default: root) |
| `-P, --passwd <PASSWD>` | SSH password |
| `-i, --key <KEY>` | Path to private key file |
| `-p, --port <PORT>` | SSH port (default: 22) |
| `--scope <SCOPE>` | IPv6 scope ID (e.g., interface name or number) |
| `-a, --auth <AUTH>` | Authentication method (password, key, none) |

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Static musl build (for distribution)
cargo build --release --target x86_64-unknown-linux-musl
```

The musl build produces a fully static binary with no dynamic library dependencies, suitable for deployment in containerized or minimal environments.

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_session_builder

# Run with output
cargo test -- --nocapture
```

## Development Status

`simple_ssh` is currently in early development.
While the core functionalities are implemented, the API may undergo changes.
Contributions, issues, and feature requests are welcome!

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgements

[russh](https://github.com/Eugeny/russh) - The underlying SSH library used for client and server implementations.
