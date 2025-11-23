-- Simplified extraction for testing
SELECT jsonb_build_object(
    'metadata', jsonb_build_object(
        'generated_at', NOW(),
        'total_articles', (SELECT COUNT(*) FROM articles),
        'description', 'News article-source visualization'
    ),
    'sources', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', src.id,
                'name', src.source_name,
                'domain', src.domain,
                'article_count', article_counts.count
            )
        )
        FROM sources src
        JOIN (
            SELECT sourceid, COUNT(*) as count
            FROM articles
            GROUP BY sourceid
            HAVING COUNT(*) >= 10
        ) article_counts ON src.id = article_counts.sourceid
    ),
    'articles', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', a.hashid,
                'title', a.title,
                'date', a.ct,
                'url', a.url,
                'source_id', a.sourceid
            )
        )
        FROM (
            SELECT hashid, title, ct, url, sourceid
            FROM articles
            ORDER BY ct DESC
            LIMIT 100
        ) a
    ),
    'relationships', (
        SELECT jsonb_agg(
            jsonb_build_object(
                'article_id', a.hashid,
                'source_id', a.sourceid
            )
        )
        FROM (
            SELECT hashid, sourceid
            FROM articles
            ORDER BY ct DESC
            LIMIT 100
        ) a
    )
) as data;