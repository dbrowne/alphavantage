-- Extract crypto data for heatmap visualization
-- Uses symbols table with sec_type = 'Cryptocurrency'

SELECT jsonb_build_object(
    'metadata', jsonb_build_object(
        'generated_at', NOW(),
        'total_cryptos', (SELECT COUNT(*) FROM symbols WHERE sec_type = 'Cryptocurrency'),
        'description', 'Cryptocurrency heatmap by market cap and 24h price change'
    ),
    'cryptos', (
        SELECT jsonb_agg(crypto_data)
        FROM (
            SELECT jsonb_build_object(
                'symbol', s.symbol,
                'name', s.name,
                'market_cap', b.market_cap,
                'market_cap_rank', b.market_cap_rank,
                'current_price', b.current_price,
                'volume_24h', b.volume_24h,
                'price_change_24h', m.price_change_24h,
                'price_change_pct_24h', m.price_change_pct_24h,
                'price_change_pct_7d', m.price_change_pct_7d,
                'price_change_pct_30d', m.price_change_pct_30d,
                'circulating_supply', b.circulating_supply,
                'last_updated', b.last_updated,
                'coingecko_id', sm_cg.source_identifier,
                'coinmarketcap_id', sm_cmc.source_identifier
            ) as crypto_data
            FROM symbols s
            LEFT JOIN crypto_overview_basic b ON s.sid = b.sid
            LEFT JOIN crypto_overview_metrics m ON s.sid = m.sid
            LEFT JOIN symbol_mappings sm_cg ON s.sid = sm_cg.sid AND sm_cg.source_name = 'coingecko'
            LEFT JOIN symbol_mappings sm_cmc ON s.sid = sm_cmc.sid AND sm_cmc.source_name = 'coinmarketcap'
            WHERE s.sec_type = 'Cryptocurrency'
                AND s.priority < 9999999
                AND b.market_cap IS NOT NULL
                AND b.current_price IS NOT NULL
                AND b.market_cap > 0
            ORDER BY b.market_cap_rank NULLS LAST
            LIMIT 100
        ) ranked_cryptos
    )
) as data;
