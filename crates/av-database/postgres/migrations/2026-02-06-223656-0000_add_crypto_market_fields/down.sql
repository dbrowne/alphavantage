-- Undo crypto market fields migration

ALTER TABLE crypto_overview_metrics
DROP COLUMN IF EXISTS high_24h,
DROP COLUMN IF EXISTS low_24h,
DROP COLUMN IF EXISTS market_cap_change_24h,
DROP COLUMN IF EXISTS market_cap_change_pct_24h;

ALTER TABLE crypto_overview_basic
DROP COLUMN IF EXISTS image_url,
DROP COLUMN IF EXISTS market_cap_rank_rehyp;