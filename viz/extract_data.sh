#!/bin/bash

# Extract news relationship data from PostgreSQL and save to JSON
# Usage: ./extract_data.sh

set -e

# Database connection parameters
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-6433}"
DB_NAME="${DB_NAME:-sec_master}"
DB_USER="${DB_USER:-ts_user}"
DB_PASS="${DB_PASS:-dev_pw}"

echo "Extracting news relationship data..."
echo "Database: $DB_NAME@$DB_HOST:$DB_PORT"

# Run the query and save to JSON file (using derived relationships from article titles)
PGPASSWORD="$DB_PASS" psql \
    -h "$DB_HOST" \
    -p "$DB_PORT" \
    -U "$DB_USER" \
    -d "$DB_NAME" \
    -t \
    -A \
    -f extract_derived_symbols.sql \
    -o news_data.json

# Pretty print the JSON
if command -v jq &> /dev/null; then
    echo "Formatting JSON with jq..."
    jq '.' news_data.json > news_data_formatted.json
    mv news_data_formatted.json news_data.json
fi

# Get file size
FILE_SIZE=$(du -h news_data.json | cut -f1)

# Count records
if command -v jq &> /dev/null; then
    SYMBOL_COUNT=$(jq '.symbols | length' news_data.json)
    ARTICLE_COUNT=$(jq '.articles | length' news_data.json)
    RELATIONSHIP_COUNT=$(jq '.relationships | length' news_data.json)

    echo ""
    echo "✓ Data extraction complete!"
    echo "  File: news_data.json (${FILE_SIZE})"
    echo "  Symbols: $SYMBOL_COUNT"
    echo "  Articles: $ARTICLE_COUNT"
    echo "  Relationships: $RELATIONSHIP_COUNT"
else
    echo ""
    echo "✓ Data extraction complete!"
    echo "  File: news_data.json (${FILE_SIZE})"
fi

echo ""
echo "Open index.html in a web browser to view the visualization"
