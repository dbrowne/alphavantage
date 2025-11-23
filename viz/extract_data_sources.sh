#!/bin/bash

# Extract article-source relationship data (alternative when article_symbols is empty)
# Usage: ./extract_data_sources.sh

set -e

# Database connection parameters
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-6433}"
DB_NAME="${DB_NAME:-sec_master}"
DB_USER="${DB_USER:-ts_user}"
DB_PASS="${DB_PASS:-dev_pw}"

echo "Extracting article-source relationship data..."
echo "Database: $DB_NAME@$DB_HOST:$DB_PORT"

# Run the query and save to JSON file
PGPASSWORD="$DB_PASS" psql \
    -h "$DB_HOST" \
    -p "$DB_PORT" \
    -U "$DB_USER" \
    -d "$DB_NAME" \
    -t \
    -A \
    -f extract_articles_by_source.sql \
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
    SOURCE_COUNT=$(jq '.data.sources | length' news_data.json)
    ARTICLE_COUNT=$(jq '.data.articles | length' news_data.json)
    RELATIONSHIP_COUNT=$(jq '.data.relationships | length' news_data.json)

    echo ""
    echo "✓ Data extraction complete!"
    echo "  File: news_data.json (${FILE_SIZE})"
    echo "  Sources: $SOURCE_COUNT"
    echo "  Articles: $ARTICLE_COUNT"
    echo "  Relationships: $RELATIONSHIP_COUNT"
else
    echo ""
    echo "✓ Data extraction complete!"
    echo "  File: news_data.json (${FILE_SIZE})"
fi

echo ""
echo "Open index_sources.html in a web browser to view the visualization"