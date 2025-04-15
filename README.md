# simple_ssh

**`simple_ssh`** is a lightweight, asynchronous Rust library that simplifies SSH operations such as executing remote commands and transferring files via SCP. Built on top of the [`russh`](https://github.com/Eugeny/russh) and [`russh-keys`](https://github.com/Eugeny/russh) crates, it offers a streamlined API for secure and efficient SSH interactions.

## Features

- Asynchronous SSH client operations using `tokio`.
- Execute remote shell commands with ease.
- Transfer files securely using the SCP protocol.
- Minimalistic and focused API design.

## Getting Started

### Installation
Add `simple_ssh` to your project's `Cargo.toml`

```toml
[dependencies]
simple_ssh = { git = "https://github.com/juliankahlert/simple_ssh", tag = "v0.1.0" }
```

Replace `"0.1.0"` with the desired version.

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
        .build();

    let mut ssh = session?.connect().await?;
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
        .build();

    let mut ssh = session?.connect().await?;
    println!("Connected");

    ssh.scp("local_file.txt", "/remote/path/remote_file.txt").await?;
    println!("File transferred successfully.");
    Ok(())
}
```

> *Note: Ensure that the SSH server on the remote host supports SCP and that the user has the necessary permissions*

## Development Status

`simple_ssh` is currently in early development.
While the core functionalities are implemented, the API may undergo changes.
Contributions, issues, and feature requests are welome!

## Licnse

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for deails.

## Acknowledgements
[russh](https://github.com/Eugeny/russh) - The underlying SSH library used for client and server implementations.
