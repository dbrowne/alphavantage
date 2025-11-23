-- Complete news visualization with article-symbol relationships derived from titles
-- This eliminates the need for the article_symbols table

SELECT jsonb_build_object(
    'metadata', jsonb_build_object(
        'generated_at', NOW(),
        'total_articles', (SELECT COUNT(*) FROM articles),
        'description', 'News article-symbol visualization (derived from article titles)',
        'derivation_method', 'regex extraction from titles'
    ),
    'symbols', (
        WITH extracted_tickers AS (
            SELECT DISTINCT
                (regexp_matches(title, '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?([A-Z]{1,5})\s*\)', 'g'))[1] as ticker
            FROM articles
        ),
        symbol_stats AS (
            SELECT
                s.sid,
                s.symbol,
                s.name,
                COUNT(a.hashid) as article_count
            FROM symbols s
            INNER JOIN extracted_tickers et ON s.symbol = et.ticker
            INNER JOIN articles a ON a.title ~ ('\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?' || s.symbol || '\s*\)')
            WHERE s.sec_type = 'Equity'
            GROUP BY s.sid, s.symbol, s.name
            HAVING COUNT(a.hashid) >= 3
            ORDER BY COUNT(a.hashid) DESC
            LIMIT 50
        )
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', sid,
                'symbol', symbol,
                'name', name,
                'article_count', article_count
            )
        )
        FROM symbol_stats
    ),
    'articles', (
        -- Select recent articles that have ticker symbols
        WITH recent_articles AS (
            SELECT hashid, title, ct, url, sourceid
            FROM articles
            WHERE title ~ '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?[A-Z]{1,5}\s*\)'
            ORDER BY ct DESC
            LIMIT 1000
        )
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', hashid,
                'title', title,
                'date', ct,
                'url', url,
                'source_id', sourceid
            )
        )
        FROM recent_articles
    ),
    'relationships', (
        WITH top_symbols AS (
            -- Get the same symbols as in the symbols section above
            WITH extracted_tickers AS (
                SELECT DISTINCT
                    (regexp_matches(title, '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?([A-Z]{1,5})\s*\)', 'g'))[1] as ticker
                FROM articles
            )
            SELECT s.sid
            FROM symbols s
            INNER JOIN extracted_tickers et ON s.symbol = et.ticker
            INNER JOIN articles a ON a.title ~ ('\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?' || s.symbol || '\s*\)')
            WHERE s.sec_type = 'Equity'
            GROUP BY s.sid
            HAVING COUNT(a.hashid) >= 3
            ORDER BY COUNT(a.hashid) DESC
            LIMIT 50
        ),
        recent_articles AS (
            -- Same article set as above
            SELECT hashid
            FROM articles
            WHERE title ~ '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?[A-Z]{1,5}\s*\)'
            ORDER BY ct DESC
            LIMIT 1000
        ),
        extracted_relations AS (
            SELECT DISTINCT
                a.hashid as article_id,
                s.sid as symbol_id
            FROM articles a
            INNER JOIN recent_articles ra ON a.hashid = ra.hashid
            CROSS JOIN LATERAL regexp_matches(a.title, '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?([A-Z]{1,5})\s*\)', 'g') AS ticker_match
            INNER JOIN symbols s ON s.symbol = ticker_match[1]
            INNER JOIN top_symbols ts ON s.sid = ts.sid
            WHERE s.sec_type = 'Equity'
        )
        SELECT jsonb_agg(
            jsonb_build_object(
                'article_id', article_id,
                'symbol_id', symbol_id
            )
        )
        FROM extracted_relations
    ),
    'sources', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', src.id,
                'name', src.source_name,
                'domain', src.domain
            )
        )
        FROM sources src
        WHERE EXISTS (
            SELECT 1 FROM articles a
            WHERE a.sourceid = src.id
                AND a.title ~ '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?[A-Z]{1,5}\s*\)'
        )
    )
) as data;