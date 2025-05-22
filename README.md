# CipherStream

CipherStream is a secure P2P file transfer application built with Rust and libp2p. It allows users to send files directly to other peers on the network with **automatic transport-level encryption** via libp2p's Noise protocol.

## Features

- **Peer-to-Peer Architecture**: Direct file transfers between peers without any central server
- **Automatic Transport Security**: Built-in encryption using libp2p's Noise protocol for all communications
- **Auto Discovery**: Local network peer discovery using mDNS
- **Reliable Transfers**: Chunked file transfer with progress tracking
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **Command-Line Interface**: Easy-to-use commands for sending and receiving files

## Security

CipherStream leverages **libp2p's Noise protocol** for automatic transport-level encryption. This means:

- All communications between peers are automatically encrypted
- No need to manually enable encryption - it's always on
- Uses industry-standard cryptographic protocols
- Peer authentication and secure key exchange are handled automatically
- Transport security is transparent to the user

## Installation

### Prerequisites

- Rust and Cargo (1.54.0 or later)

### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/cipherstream.git
   cd cipherstream
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. The binary will be available at `target/release/cipherstream`

## Usage

### Starting a Node

To start a CipherStream node that can receive files:

```bash
cipherstream start --port 8000
```

Options:
- `--port <PORT>`: Port number to listen on (default: 8000)
- `--data-dir <DIR>`: Directory to store application data (default: ~/.cipherstream)

### Sending a File

To send a file to another peer:

```bash
cipherstream send --peer <PEER_ID> --file <FILE_PATH>
```

Options:
- `--peer <PEER_ID>`: The peer ID of the receiving node
- `--file <FILE_PATH>`: Path to the file to send

**Note**: All file transfers are automatically encrypted via libp2p's transport security.

### Examples

1. Start a node on port 8000:
   ```bash
   cipherstream start --port 8000
   ```

2. Send a file to a peer:
   ```bash
   cipherstream send --peer 12D3KooWB8rRTvkEnEpSvfGYEUkgpNtnEfwYzpnqCMgTRo7LghDz --file ~/Documents/report.pdf
   ```

## Architecture

CipherStream is built on top of the libp2p networking stack and uses several components:

- **Network**: Handles connections, peer discovery and communication using libp2p with Noise protocol for security
- **Protocol**: Defines the messages exchanged during file transfers
- **File Transfer**: Manages the actual file transfer operations with chunking and progress tracking
- **Discovery**: Uses mDNS for local peer discovery and Kademlia DHT for broader network discovery

The application uses asynchronous Rust with Tokio runtime for handling concurrent operations.

## Testing

CipherStream has an extensive test suite covering core functionality. To run the tests:

```bash
# Run the automated test script
./test.sh unit          # Run unit tests
./test.sh integration   # Run integration tests
./test.sh benchmarks    # Run performance benchmarks
./test.sh all           # Run all tests
```

### Test Coverage

- **Unit Tests**: Cover crypto operations, message serialization, network configuration, and more
- **Integration Tests**: Test actual file transfers between nodes
- **Performance Tests**: Benchmark crypto operations at different data sizes

## Known Issues

- Integration tests may occasionally fail due to port conflicts
- mDNS discovery can be slow on some networks

## License

[MIT License](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request 