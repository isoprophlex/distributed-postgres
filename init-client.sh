#!/bin/bash

# Check if cluster name is provided
if [ -z "$1" ]; then
    echo "[init-client] Error: Cluster name is required."
    exit 1
fi

# Assign the first argument as the cluster name
CLUSTER_NAME=$1

# Run init-server.sh with cluster name and "c" as the node type
./init-server.sh "$CLUSTER_NAME" c