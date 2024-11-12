#!/bin/bash

# Ask for the database cluster name if not provided
if [ -z "$3" ]; then
    read -p "Enter the name of the database cluster to start: " DB_CLUSTER_NAME
else
    DB_CLUSTER_NAME=$3
fi

# Define paths
ROOT_DIR=$(pwd)
SHARDING_DIR="$ROOT_DIR/sharding"
PG_CTL_DIR="$ROOT_DIR/src/bin/pg_ctl"
POSTGRES_EXECUTABLE="$ROOT_DIR/src/backend/postgres"
CLUSTERS_DIR="$ROOT_DIR/clusters"
DB_DIR="$CLUSTERS_DIR/$DB_CLUSTER_NAME"
LOG_FILE="$CLUSTERS_DIR/logfile"
CONFIG_FILE="$SHARDING_DIR/src/node/config/nodes_config.yaml" # Path to config.yaml

# Check for additional argument
START_PSQL=$1
NODE_TYPE=$2

# If we're on OS X, make sure that globals aren't stripped out.
if [ "$(uname)" == "Darwin" ]; then
    export LDFLAGS="-Wl,-no_pie"
fi

./build-release.sh
echo "[init-server] Building the project..."
make

echo "[init-server] Copying postgres executable to pg_ctl directory..."
cd $PG_CTL_DIR
rm postgres
cd $ROOT_DIR
cp $POSTGRES_EXECUTABLE $PG_CTL_DIR

# Function to check if a port is available
port_available() {
    local port=$1
    # Check if port is in use
    if nc -z localhost $port; then
        echo "[init-server] Port $port is in use."
        return 1  # Port is in use
    else
        echo "[init-server] Port $port is available."
        return 0  # Port is available
    fi
}

# Read ports from config.yaml using the Python script
ports=($(python3 parse_config_yaml.py $CONFIG_FILE))

# Find an available port
selected_port=""
for port in "${ports[@]}"; do
    if port_available $port; then
        selected_port=$port
        break
    fi
done

if [ -z "$selected_port" ]; then
    echo "[init-server] Error: No available ports found in config.yaml"
    exit 1
fi

echo "[init-server] Starting PostgreSQL server on port $selected_port for cluster $DB_CLUSTER_NAME with node type $NODE_TYPE..."
cd $PG_CTL_DIR
./pg_ctl -D $DB_DIR -l $LOG_FILE -o "-p $selected_port" start

# Check if pg_ctl ran successfully
if [ $? -ne 0 ]; then
    echo "[init-server] Error: Failed to start PostgreSQL server."
    exit 1
fi

# If "start" argument is provided, run start-psql.sh
if [ "$START_PSQL" == "start" ]; then
    echo "[init-server] Calling start-psql.sh with nodeType $NODE_TYPE and port $selected_port..."
    cd $ROOT_DIR
    ./start-psql.sh $selected_port $NODE_TYPE
fi
