#!/bin/bash

# Paths to the local project files
LOCAL_SERVICE_FILE="usecases/bench-sync.service"

# Paths to the systemd unit files
SERVICE_PATH="/etc/systemd/system/bench-sync.service"

# Check if the local service unit file exists
if [ ! -f "$LOCAL_SERVICE_FILE" ]; then
  echo "Error: $LOCAL_SERVICE_FILE does not exist. Please create the service unit file before running this script."
  exit 1
fi

sudo cp $LOCAL_SERVICE_FILE $SERVICE_PATH

sudo systemctl daemon-reload
sudo systemctl enable bench-sync.service
sudo systemctl start bench-sync.service

echo "bench loop service has been installed successfully"

