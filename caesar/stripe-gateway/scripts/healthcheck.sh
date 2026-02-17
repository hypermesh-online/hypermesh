#!/bin/sh

# Health check script for Docker container
# Checks if the service is responding to HTTP requests

# Configuration
HOST="localhost"
PORT="9292"
TIMEOUT="10"
ENDPOINT="/health"

# Make HTTP request to health endpoint
response=$(curl -s -w "%{http_code}" -o /dev/null --max-time $TIMEOUT "http://$HOST:$PORT$ENDPOINT")

# Check if response is 200 OK
if [ "$response" = "200" ]; then
    echo "Health check passed - Service is healthy"
    exit 0
else
    echo "Health check failed - HTTP $response"
    exit 1
fi