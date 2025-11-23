-- Extract article-source relationship data for visualization
-- This shows which sources publish which articles (alternative when article_symbols is empty)

WITH recent_articles AS (
    -- Get articles from last 30 days
    SELECT
        a.hashid,
        a.title,
        a.ct,
        a.url,
        a.sourceid,
        src.source_name,
        src.domain
    FROM articles a
    JOIN sources src ON a.sourceid = src.id
    WHERE a.ct >= CURRENT_DATE - INTERVAL '30 days'
    LIMIT 100  -- Limit for POC
),
source_stats AS (
    -- Get source statistics
    SELECT
        src.id,
        src.source_name,
        src.domain,
        COUNT(DISTINCT a.hashid) as article_count
    FROM sources src
    JOIN articles a ON src.id = a.sourceid
    WHERE a.ct >= CURRENT_DATE - INTERVAL '30 days'
    GROUP BY src.id, src.source_name, src.domain
    HAVING COUNT(DISTINCT a.hashid) >= 3
)
-- Build JSON output
SELECT jsonb_build_object(
    'metadata', jsonb_build_object(
        'generated_at', NOW(),
        'date_range_start', CURRENT_DATE - INTERVAL '30 days',
        'date_range_end', CURRENT_DATE,
        'description', 'Article-source relationships (last 30 days)',
        'note', 'This visualization shows articles grouped by source. To see symbol relationships, load data into article_symbols table.'
    ),
    'sources', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', ss.id,
                'name', ss.source_name,
                'domain', ss.domain,
                'article_count', ss.article_count
            )
        )
        FROM source_stats ss
    ),
    'articles', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', ra.hashid,
                'title', ra.title,
                'date', ra.ct,
                'url', ra.url,
                'source_id', ra.sourceid,
                'source_name', ra.source_name
            )
        )
        FROM recent_articles ra
    ),
    'relationships', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'article_id', ra.hashid,
                'source_id', ra.sourceid,
                'date', ra.ct
            )
        )
        FROM recent_articles ra
    )
) as data;
