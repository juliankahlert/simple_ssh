# simple_ssh

[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/juliankahlert/simple_ssh)](https://rust-reportcard.xuri.me/report/github.com/juliankahlert/simple_ssh)

**`simple_ssh`** is a lightweight, asynchronous Rust library that simplifies SSH operations such as executing remote commands, transferring files via SCP, and interactive PTY sessions. Built on top of the [`russh`](https://github.com/Eugeny/russh) and [`russh-keys`](https://github.com/Eugeny/russh) crates, it offers a streamlined API for secure and efficient SSH interactions.

## Features

- Asynchronous SSH client operations using `tokio`
- Execute remote shell commands (`cmd`, `exec`, `system`)
- Transfer files securely using the SCP protocol
- Interactive PTY shell sessions with raw mode and auto-resize
- Programmatic PTY sessions via `PtyHandle` for embedding in TUIs
- Terminal multiplexer support (1x2, 2x1, 2x2 layouts)
- IPv6 link-local address support with scope ID
- Authentication modes: public key, password, and none
- SSH certificate support for key authentication
- Minimalistic and focused API design

## Installation

### As a Library

Add `simple_ssh` to your project's `Cargo.toml`:

```toml
[dependencies]
simple_ssh = "0.1.3"
```

### CLI Tools

To install the CLI binaries (`simple-ssh` and `simple-scp`), use the `cli` feature:

```bash
cargo install simple_ssh --features cli
```

This installs both `simple-ssh` and `simple-scp` binaries to your cargo bin directory.

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

    // Execute a shell command
    let code = ssh.cmd("ls -la").await?;
    println!("Exitcode: {:?}", code);

    // Execute with arguments (properly escaped)
    let code = ssh.exec(&["ls", "-la", "/tmp"].iter().map(|s| s.to_string()).collect()).await?;
    println!("Exitcode: {code}");

    // Execute via sh -c (like system())
    let code = ssh.system("echo $HOME && ls -la").await?;
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

### Programmatic PTY Sessions

For non-interactive PTY sessions where you control the I/O (e.g., embedding in a TUI):

```rust
use simple_ssh::Session;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::init()
        .with_host("192.168.0.100")
        .with_user("root")
        .with_passwd("toor")
        .build()?;

    let mut ssh = session.connect().await?;

    // Open a PTY handle for programmatic control
    let mut handle = ssh
        .pty_builder()
        .with_term("xterm-256color")
        .with_size(80, 24)
        .open()
        .await?;

    // Send input
    handle.write(b"ls -la\n").await?;

    // Exit the shell so handle.read() returns None and handle.wait() completes
    handle.write(b"exit\n").await?;

    // Read output - loop ends when shell exits and handle.read() returns None
    while let Some(data) = handle.read().await {
        println!("Received: {:?}", data);
    }

    // Wait for exit status
    let status = handle.wait().await?;
    println!("Exit status: {:?}", status.code());

    Ok(())
}
```

## CLI Tools

Two example binaries are provided for command-line use:

### simple-ssh

Interactive SSH client with PTY support and terminal multiplexing:

```bash
# Interactive shell
simple-ssh -H 192.168.1.1 -u root -P password

# Execute command
simple-ssh -H server.example.com -u admin -P secret "uname -a"

# With private key
simple-ssh -H server.example.com -i /path/to/key

# IPv6 link-local with scope
simple-ssh -H fe80::1%eth0 -u root -P password

# Terminal multiplexer (2 panes stacked vertically)
simple-ssh -H 192.168.1.1 -u root -P password --mux 1x2

# Terminal multiplexer (2 panes side by side)
simple-ssh -H 192.168.1.1 -u root -P password --mux 2x1

# Terminal multiplexer (4 panes in 2x2 grid)
simple-ssh -H 192.168.1.1 -u root -P password --mux 2x2
```

### simple-scp

File transfer utility:

```bash
# Transfer file using password authentication
simple-scp -H 192.168.1.1 -u root -P password /local/file.txt /remote/path.txt

# Transfer file using private key
simple-scp -H server.example.com -i /path/to/key /local/file.txt /remote/path.txt

# With custom port
simple-scp -H 192.168.1.1 -p 2222 -u admin -P secret /local/file.txt /remote/path.txt
```

### CLI Options

#### simple-ssh Options

| Option | Description |
|--------|-------------|
| `-H, --host <HOST>` | SSH host to connect to (required) |
| `-u, --user <USER>` | SSH username (default: root) |
| `-P, --passwd <PASSWD>` | SSH password |
| `-i, --key <KEY>` | Path to private key file |
| `-p, --port <PORT>` | SSH port (default: 22) |
| `--scope <SCOPE>` | IPv6 scope ID (e.g., interface name or number) |
| `-a, --auth <AUTH>` | Authentication method (password, key, none) |
| `--mux <MODE>` | Terminal multiplexer mode: 1x2, 2x1, or 2x2 |

#### simple-scp Options

| Option | Description |
|--------|-------------|
| `-H, --host <HOST>` | SSH host to connect to (required) |
| `-u, --user <USER>` | SSH username (default: root) |
| `-P, --passwd <PASSWD>` | SSH password |
| `-i, --key <KEY>` | Path to private key file |
| `-p, --port <PORT>` | SSH port (default: 22) |
| `--scope <SCOPE>` | IPv6 scope ID (e.g., interface name or number) |
| `-a, --auth <AUTH>` | Authentication method (password, key, none) |
| `<LOCAL>` | Local file path to upload |
| `<REMOTE>` | Remote destination path |

## Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Build CLI examples
cargo build --bin simple-ssh --features cli
cargo build --bin simple-scp --features cli

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

`simple_ssh` is actively developed and provides a stable API for common SSH operations.
The core functionalities including command execution, SCP file transfer, and PTY sessions
are fully implemented and tested. Contributions, issues, and feature requests are welcome!

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgements

[russh](https://github.com/Eugeny/russh) - The underlying SSH library used for client and server implementations.
