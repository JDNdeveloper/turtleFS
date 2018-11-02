turtleFS
========================

**Author:** Jayden Navarro

**Email:** jdndeveloper@gmail.com

**LinkedIn:** [Jayden Navarro](https://www.linkedin.com/in/jaydennavarro)

**Twitter:** [@JaydenNavarro](https://twitter.com/JaydenNavarro)

**Google+:** [Jayden Navarro](https://plus.google.com/u/0/+JaydenNavarro/posts)

**GitHub:** [JDNdeveloper](http://www.github.com/JDNdeveloper)

## Description:

`turtleFS` is a distributed filestore written in Rust. It currently only supports distributed reads across single/multiple servers, but more functionality is planned. Windows/Linux/Mac are all supported.

All metadata and data files are stored within the `turtlefs-root` directory (user configurable). The data files are located in `turtlefs-root/store/`, along with the file manifest `turtlefs-root/store/file_store.yaml` which stores which files are provided by which nodes (a node is an `IP:TCP_PORT` pair). The root nodes are stored in `turtlefs-root/nodes.yaml`, which the client uses to retrieve the file manifest.

## Trying it out

An example `turtlefs-root` is provided in `src/example_turtlefs_root`, which contains `nodes.yaml`, `file_store.yaml`, and two data files: `hello.txt` and `hello.zip`.

The following sections will walk you through setting up a `turtlefs-root`, and running a server and client.

### Setting up `turtlefs-root`

Create an empty directory to use as the `turtlefs-root`.

Create the `nodes.yaml` file under `turtlefs-root`, and add the root nodes to it (i.e. the server nodes which contain replicas of `file_store.yaml`).

**Example:**
```
- 192.168.0.155:5550
- 192.168.0.155:5551
- 192.168.0.155:5552
```

Now create a `store` subdirectory.

Create the `file_store.yaml` file under `turtlefs-root/store`, and add the files in the file store, along with their crc32 checksum and the nodes that provide the file.

**Example:**
```
/hello.txt:
  checksum: DB588331
  nodes:
    - 192.168.0.155:5550
/hello.zip:
  checksum: B57EB130
  nodes:
    - 192.168.0.155:5550
    - 192.168.0.155:5551
```

Note that all files in the `turtleFS` system are rooted at `turtlefs-root/store/`, and should be provided/requested relative to this root.

### Running turtleFS server

The server takes two parameters, the `turtlefs-root` and the `node-id` (`IP:TCP_PORT`).

You can start as many servers as you like on a given machine as long as the port is different. In the above deployment we are using three instances of the server with port range `5550` to `5552`.

When files are requested, the request is logged and outputted by the server. Any request errors encountered are outputted, but do not crash the server.

#### Linux

```
Hola! ~/turtleFS $ ./target/debug/server.exe "~/turtleFS/src/example_turtlefs_root" 192.168.0.155:5550
```

#### Windows

```
Hola! ~/turtleFS $ ./target/debug/server.exe "C:\Users\...\turtleFS\src\example_turtlefs_root" 192.168.0.155:5550
```

### Running turtleFS client

The client takes two parameters, the `turtlefs-root` and the file path that is being requested.

#### Linux

```
Hola! ~/turtleFS $ ./target/debug/client.exe "~/turtleFS/src/example_turtlefs_root" /hello.txt
Hello
World
```

#### Windows

When running in Windows, the `turtlefs-root` must be provided in standard Windows path format, but the request path (which is relative to `turtlefs-root/store/`) should always use the `/` path separator.

```
Hola! ~/turtleFS $ ./target/debug/client.exe "C:\Users\...\turtleFS\src\example_turtlefs_root" /hello.txt
Hello
World
```
