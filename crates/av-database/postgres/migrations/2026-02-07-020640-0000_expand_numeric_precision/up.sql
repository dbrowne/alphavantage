-- Expand numeric precision to prevent overflow
-- Change from NUMERIC(20,8) to NUMERIC(38,8) for all constrained columns

-- First drop dependent views
DROP VIEW IF EXISTS crypto_full_view;
DROP VIEW IF EXISTS crypto_overviews;

-- crypto_overview_basic
ALTER TABLE crypto_overview_basic
  ALTER COLUMN volume_change_24h TYPE NUMERIC(38,8),
  ALTER COLUMN current_price TYPE NUMERIC(38,8),
  ALTER COLUMN circulating_supply TYPE NUMERIC(38,8),
  ALTER COLUMN total_supply TYPE NUMERIC(38,8),
  ALTER COLUMN max_supply TYPE NUMERIC(38,8);

-- crypto_overview_metrics
ALTER TABLE crypto_overview_metrics
  ALTER COLUMN price_change_24h TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_24h TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_7d TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_14d TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_30d TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_60d TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_200d TYPE NUMERIC(38,8),
  ALTER COLUMN price_change_pct_1y TYPE NUMERIC(38,8),
  ALTER COLUMN ath TYPE NUMERIC(38,8),
  ALTER COLUMN ath_change_percentage TYPE NUMERIC(38,8),
  ALTER COLUMN atl TYPE NUMERIC(38,8),
  ALTER COLUMN atl_change_percentage TYPE NUMERIC(38,8),
  ALTER COLUMN roi_times TYPE NUMERIC(38,8),
  ALTER COLUMN roi_percentage TYPE NUMERIC(38,8);

-- Recreate views
CREATE VIEW crypto_overviews AS
SELECT
  b.sid,
  b.symbol,
  b.name,
  b.slug,
  b.description,
  b.market_cap_rank,
  b.market_cap,
  b.fully_diluted_valuation,
  b.volume_24h,
  b.volume_change_24h,
  b.current_price,
  m.price_change_24h,
  m.price_change_pct_24h,
  m.price_change_pct_7d,
  m.price_change_pct_14d,
  m.price_change_pct_30d,
  m.price_change_pct_60d,
  m.price_change_pct_200d,
  m.price_change_pct_1y,
  m.ath,
  m.ath_date,
  m.ath_change_percentage,
  m.atl,
  m.atl_date,
  m.atl_change_percentage,
  m.roi_times,
  m.roi_currency,
  m.roi_percentage,
  b.circulating_supply,
  b.total_supply,
  b.max_supply,
  b.last_updated,
  b.c_time,
  b.m_time
FROM crypto_overview_basic b
LEFT JOIN crypto_overview_metrics m ON b.sid = m.sid;

CREATE VIEW crypto_full_view AS
SELECT
  s.sid,
  s.symbol,
  s.name,
  s.sec_type,
  cob.market_cap_rank,
  cob.market_cap,
  cob.current_price,
  cob.volume_24h,
  com.price_change_pct_24h,
  com.price_change_pct_7d,
  com.price_change_pct_30d,
  cs.coingecko_score,
  cs.developer_score,
  cs.community_score,
  ct.is_defi,
  ct.is_stablecoin,
  ct.blockchain_platform,
  cm.exchange,
  cm.base,
  cm.target
FROM symbols s
LEFT JOIN crypto_overview_basic cob ON s.sid = cob.sid
LEFT JOIN crypto_overview_metrics com ON s.sid = com.sid
LEFT JOIN crypto_social cs ON s.sid = cs.sid
LEFT JOIN crypto_technical ct ON s.sid = ct.sid
LEFT JOIN crypto_markets cm ON s.sid = cm.sid
WHERE s.sec_type = 'Cryptocurrency';
