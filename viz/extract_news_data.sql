-- Extract news relationship data for visualization
-- This query gets symbol-article relationships with sentiment data
-- Limited to last 30 days and top symbols for POC

WITH top_symbols AS (
    -- Get top 25 most mentioned symbols
    SELECT
        s.sid,
        s.symbol,
        s.name,
        s.sec_type,
        COUNT(DISTINCT asym.articleid) as article_count
    FROM symbols s
    JOIN article_symbols asym ON s.sid = asym.sid
    JOIN articles a ON asym.articleid = a.hashid
    WHERE a.ct >= CURRENT_DATE - INTERVAL '30 days'
    GROUP BY s.sid, s.symbol, s.name, s.sec_type
    ORDER BY article_count DESC
    LIMIT 25
),
article_data AS (
    -- Get articles mentioning these top symbols
    SELECT DISTINCT
        a.hashid,
        a.title,
        a.ct,
        a.url,
        src.source_name,
        src.domain
    FROM articles a
    JOIN sources src ON a.sourceid = src.id
    JOIN article_symbols asym ON a.hashid = asym.articleid
    WHERE asym.sid IN (SELECT sid FROM top_symbols)
        AND a.ct >= CURRENT_DATE - INTERVAL '30 days'
)
-- Main query: combine symbols, articles, and relationships
SELECT jsonb_build_object(
    'metadata', jsonb_build_object(
        'generated_at', NOW(),
        'date_range_start', CURRENT_DATE - INTERVAL '30 days',
        'date_range_end', CURRENT_DATE,
        'description', 'News relationship data for top 25 symbols (last 30 days)'
    ),
    'symbols', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'sid', ts.sid,
                'symbol', ts.symbol,
                'name', ts.name,
                'sec_type', ts.sec_type,
                'article_count', ts.article_count
            )
        )
        FROM top_symbols ts
    ),
    'articles', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', ad.hashid,
                'title', ad.title,
                'date', ad.ct,
                'url', ad.url,
                'source', ad.source_name,
                'domain', ad.domain
            )
        )
        FROM article_data ad
    ),
    'relationships', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'article_id', asym.articleid,
                'sid', asym.sid,
                'symbol', s.symbol,
                'relevance', COALESCE(ts.relevance, 0.5),
                'sentiment', COALESCE(ts.tsentiment, 0.0),
                'sentiment_label', COALESCE(ts.sentiment_label, 'Neutral')
            )
        )
        FROM article_symbols asym
        JOIN top_symbols ts_check ON asym.sid = ts_check.sid
        JOIN symbols s ON asym.sid = s.sid
        LEFT JOIN feeds f ON f.articleid = asym.articleid AND f.sid = asym.sid
        LEFT JOIN tickersentiments ts ON ts.feedid = f.id
        WHERE asym.articleid IN (SELECT hashid FROM article_data)
    )
) as data;
