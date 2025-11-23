-- Derive article-symbol relationships from article titles
-- Extracts ticker symbols from patterns like "( NASDAQ:NVDA )" or "( INTU )"

WITH extracted_tickers AS (
    SELECT
        a.hashid as articleid,
        a.title,
        a.ct,
        -- Extract all ticker patterns from the title
        regexp_matches(a.title, '\(\s*(?:NASDAQ:|NYSE:|NYSEARCA:|OTC:)?([A-Z]{1,5})\s*\)', 'g') as ticker_match
    FROM articles a
),
ticker_list AS (
    SELECT
        articleid,
        title,
        ct,
        ticker_match[1] as ticker_symbol
    FROM extracted_tickers
)
SELECT
    t.ticker_symbol,
    COUNT(*) as mention_count,
    json_agg(json_build_object(
        'article_id', t.articleid,
        'title', left(t.title, 100),
        'date', t.ct
    ) ORDER BY t.ct DESC) as articles
FROM ticker_list t
GROUP BY t.ticker_symbol
HAVING COUNT(*) >= 3  -- Only tickers mentioned in 3+ articles
ORDER BY mention_count DESC
LIMIT 20;