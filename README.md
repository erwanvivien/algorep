# Algorep project

## Authors

- **Hugo BOIS**
- **Nathan CABASSO**
- **Jeremy CROISET**
- **Ferdinand MOM**
- **Erwan VIVIEN**

Implementation of RAFT consensus algorithm in Rust. We followed the [paper](https://raft.github.io/raft.pdf). \
This is a school project for the course `Algorep` at EPITA (School of Engineering and Computer Science).

## Running the project

To run the project, you need to have `cargo` installed. \
Then, you can run the project by running the following command in the root directory of the project: `cargo run --release`.

You can interract with servers by using the following commands:

- `REPL <server_id> <command>` with `<server_id>` in the range of [0; node_count - 1] and `<command>` being one of the following:

  - `CRASH`: Crash the server (ignores messages)
  - `START`: Allows the clients to send messages to the servers
  - `SHUTDOWN`: Shutdown the server (stops the server, server is no longer reachable)
  - `RECOVERY`: Reload the data from the disk and remove volatile state
  - `TIMEOUT`: Force the server to start an election
  - `DISPLAY`: Display the state of the server

You can act as a client by using the following commands:

- `<command> [content]` with `<command>` being one of the following:

  - `LOAD <filename>`: Creates a file in the store
  - `GET <UID>`: Get the content of the file in the store
  - `DELETE <UID>`: Delete the key-value pair associated with the key
  - `APPEND <key> [message]`: Append the `message` to the content of the file in the store with a new line

**Note**: The client does not automaticaly retry to send a message if it fails. You need to resend the message manually. This is the design we chose for the client.

## Server configuration

The configuration file is a RON file (Rusty Object Notation) that can be found in the `config` directory and is named `config.ron`. \
What can be configured is the following:

- node_count: the number of nodes(servers) in the cluster
- election_timeout: a range of `ConfigTime` enum (see `config.rs` for more details). This range is what is used by servers to generated a random timer.
- client_count: the number of clients that will be spawned by the `main.rs` file

## Running the tests

To run the tests, you need to have `cargo` installed. \
Then, you can run the tests by running the following command in the root directory of the project: `cargo test`.

The following tests are implemented:

- `should_accept_vote`: Checks if a server accepts a vote request from another server if valid
- `should_receive_election`: Checks that a server timeouts and starts an election in the beginning of the simulation
- `should_retry_election`: Checks that a server timeouts and starts an election. If we reply with an `AppendEntries` message with a higher term, the first server should accept it
- `should_elect_first`: If two servers do not have the same election timeout, the first one to timeout should be elected
- `setup_server_client_no_recv`: Checks that a server can be setup and a client can be setup. If no interaction the client should not receive any message
- `client_send_request`: Checks that a client can send a request to a server and that the server can receive it
- `client_server_should_receive_entry`: Checks that a client can append an entry to a server and an other server receives the entry followed by empty entries.
- `should_timeout`: Checks that a server timeouts if it does not receive correct messages. e.g. if a server receives an `AppendEntries` message with a lower term, it should not reset it's timer
- `should_handle_append_entries`: Checks that a server can handle an `AppendEntries` message and reply with the correct message. This is the end-to-end test. A client generates requests to create/modify fiels, the server replicates them and applies them to the state machine. After that, if a client GET a file, it should receive the correct value.
