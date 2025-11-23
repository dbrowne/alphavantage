#!/bin/bash

# Database connection settings
DB_HOST="localhost"
DB_PORT="6433"
DB_NAME="sec_master"
DB_USER="ts_user"
DB_PASS="dev_pw"

echo "Extracting crypto heatmap data..."
echo "Database: ${DB_NAME}@${DB_HOST}:${DB_PORT}"

# Run the query and save to JSON file
PGPASSWORD="$DB_PASS" psql \
    -h "$DB_HOST" \
    -p "$DB_PORT" \
    -U "$DB_USER" \
    -d "$DB_NAME" \
    -t \
    -A \
    -f extract_crypto_heatmap.sql \
    -o crypto_data.json

# Format JSON with jq if available
if command -v jq &> /dev/null; then
    echo "Formatting JSON with jq..."
    jq '.' crypto_data.json > crypto_data.tmp && mv crypto_data.tmp crypto_data.json
fi

# Count records
if command -v jq &> /dev/null; then
    CRYPTO_COUNT=$(jq '.cryptos | length' crypto_data.json)
else
    CRYPTO_COUNT="?"
fi

FILE_SIZE=$(ls -lh crypto_data.json | awk '{print $5}')

echo ""
echo "✓ Data extraction complete!"
echo "  File: crypto_data.json ($FILE_SIZE)"
echo "  Cryptocurrencies: $CRYPTO_COUNT"
echo ""
echo "Open crypto_heatmap.html in a web browser to view the visualization"
