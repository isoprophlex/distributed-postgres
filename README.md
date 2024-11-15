Distributed Postgres
=====================================

# Contributors
- Aldana Rastrelli
- Franco Cuppari
- Nicolas Continanza

Universidad de Buenos Aires, December 2024

# About this Project
This is a distributed implementation of the PostgreSQL database management system. The project is based on the original C source code, but expanded with a Rust crate called "Sharding". The main goal of this project is to provide a distributed database system that can be used in a cluster of machines. The scope is, however, somehow limited: we took upon ourselves to achieve the goal of making this work for a simple, yet efficient, CRUD system (Create, Read, Update and Delete). 

# How to Run
## Creating Cluster
To run this locally, you need to first create some cluster databases. Each cluster will represent a different instance. You can create a cluster by running on the root of this project:
```
./create-cluster-dir.sh
```

The script will ask for a cluster name and create one in the "./clusters" directory.
If you are running only one instance in each server, you just need to create the one cluster.

You should also create a cluster for the client. This cluster should be reserved to be use only (and ever) with the client instance, because it does not have access to postgre's backend. Any data stored previously in it by a shard or a router won't be available.

## Setting the Nodes Configuration
Each node should know the other nodes in the network.
This is set manually in the "nodes_config.yaml" file (sharding/src/node/config/nodes_config.yaml).
You can follow the given example and change the IPs, ports and even given names. 
Just be careful: every server should have this file with the same content to create the sharding network. This is the telephone directory of the cluster's graph of nodes.

## Running Shards
On the root of the project, run:
```
./init-server <cluster_name>
```

This will run a shard instance.
You can call this in different terminals for every cluster you have created locally, or you can call it once if you are running in different servers.

⚠️ You should not need to run a router. The router must be a shard chosen by an election, which then changes its role in the network. But, in the rare case you need to run a router instance (for debugging, for example), you can run `./init-server <cluster_name> r`, adding an "r" at the end of the command to let it know you want to run it as a router.

## Running a Client
Finally, once the network is up and the router is elected, you may run a client to access the database. You can do so with this command:
```
./init-client.sh <cluster_name>
```

You can also run a client without the network being done initializing, but you'll just get a warning message and the terminal will be blocked, waiting for the client to find a Router.

## Sending Queries
In the client instance, you can use the terminal to query the database as you like. The abstraction provided by the router will make it seem as if the client is connected to a centralized database. Remember, the project only supports queries as: 

- INSERT
- DELETE
- DROP
- UPDATE
- CREATE
- SELECT

"WHERE" statements are also supported.

You may also query each shard individually if you like, but you'll get only the results for the data stored in said shard.
In the same way, you can query the router. This will give you a client-like behavior, getting the results for all the shards and merging them.

## Ending execution
To stop the psql execution, you can type `\q` and press enter. 
You must also run `./server-down.sh`, so it stops every postgres task running in the background.

```
# Output example of server-down.rs:
Available database clusters:
n1
n2
n3
Enter the name of the database cluster to stop (or type 'all' to stop all clusters): 
```
⚠️ You won't be able to run the instance again if you don't run server-down. You'll get a "Refused Connection" error for the given port.

<br>
<br>
<br>

Postgres Original Documentation: PostgreSQL Database Management System
=====================================

This directory contains the source code distribution of the PostgreSQL
database management system.

PostgreSQL is an advanced object-relational database management system
that supports an extended subset of the SQL standard, including
transactions, foreign keys, subqueries, triggers, user-defined types
and functions.  This distribution also contains C language bindings.

Copyright and license information can be found in the file COPYRIGHT.

General documentation about this version of PostgreSQL can be found at
<https://www.postgresql.org/docs/devel/>.  In particular, information
about building PostgreSQL from the source code can be found at
<https://www.postgresql.org/docs/devel/installation.html>.

The latest version of this software, and related software, may be
obtained at <https://www.postgresql.org/download/>.  For more information
look at our web site located at <https://www.postgresql.org/>.

## Table of Contents

- [Table of Contents](#table-of-contents)
- [How to build the project](#how-to-build-the-project)
    - [Some issues that may arise during the build process in MacOS](#some-issues-that-may-arise-during-the-build-process-in-macos)
        - [Missing tools to build the project and the documentation](#missing-tools-to-build-the-project-and-the-documentation)
        - [Missing icu4c](#missing-icu4c)


# How to build the project

Details of each step can be found in: https://www.postgresql.org/docs/devel/install-make.html

1. Clone the repository to your local machine.
2. Open the terminal and navigate to the project directory.
3. Run the following command to configure the project:
   ```
   ./configure
   ```
4. Run the following command to build the project:
   ```
   make all
   ```
5. Run the regression tests to check if the project is built correctly:
   ```
   make check
   ``` 
6. Install PostgreSQL:
    ```
    make install
    ```

## Some issues that may arise during the build process in MacOS

### Missing tools to build the project and the documentation

More details: https://www.postgresql.org/docs/current/docguide-toolsets.html#DOCGUIDE-TOOLSETS-INST-MACOS
```
brew install docbook docbook-xsl libxslt fop
```

Some of these tools require the following environment variable to be set. For Intel based machines, use this:
```
export XML_CATALOG_FILES=/usr/local/etc/xml/catalog
```

For Apple Silicon based machines, use this:
```
export XML_CATALOG_FILES=/opt/homebrew/etc/xml/catalog
```

### Missing icu4c

If you don't have icu4c installed, you can install it using Homebrew:
```
brew install icu4c
```

In some cases, you'll need to force the linking of icu4c:
```
brew link --force icu4c
```

To have icu4c in your PATH, add the following lines to your ~/.zshrc file:

```
export PATH="/opt/homebrew/opt/icu4c/bin:$PATH"
export PATH="/opt/homebrew/opt/icu4c/sbin:$PATH"
```

For compilers to find icu4c, set the following environment variables:
```
export LDFLAGS="-L/opt/homebrew/opt/icu4c/lib"
export CPPFLAGS="-I/opt/homebrew/opt/icu4c/include"
```

Finally, install pkg-config and update the PKG_CONFIG_PATH:
```
brew install pkg-config
export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c/lib/pkgconfig:$PKG_CONFIG_PATH"
```
