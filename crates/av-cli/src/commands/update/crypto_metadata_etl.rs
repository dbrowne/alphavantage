use anyhow::Result;
use diesel::prelude::*;
use serde_json::Value;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use std::str::FromStr;
use tracing::info;

use av_database_postgres::connection::establish_connection;

pub fn execute_metadata_etl(database_url: &str) -> Result<()> {
    dotenvy::dotenv().ok();
    info!("Starting crypto metadata ETL");

    let mut conn = establish_connection(database_url)?;
    let mut stats = ProcessingStats::default();

    let batch_size = 1000;
    let mut offset = 0;

    loop {
        let query = format!(r#"
            SELECT
                cm.sid,
                s.symbol,
                s.name,
                cm.market_cap_rank,
                cm.additional_data::text as json_data,
                cm.last_updated
            FROM crypto_metadata cm
            INNER JOIN symbols s ON cm.sid = s.sid
            WHERE cm.additional_data IS NOT NULL
            ORDER BY cm.sid
            LIMIT {} OFFSET {}
        "#, batch_size, offset);

        let results = diesel::sql_query(&query)
            .load::<MetadataRow>(&mut conn)?;

        if results.is_empty() {
            break; // No more records
        }

        info!("Processing batch: {} records (offset: {})", results.len(), offset);

        for row in &results {
            if let Some(json_str) = row.json_data.as_ref() {
                if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
                    // Process each table
                    if process_overview_basic(&mut conn, row, &json_value).is_ok() {
                        stats.basic_updated += 1;
                    }

                    if process_social(&mut conn, row.sid, &json_value).is_ok() {
                        stats.social_updated += 1;
                    }

                    if process_technical(&mut conn, row.sid, &json_value).is_ok() {
                        stats.technical_updated += 1;
                    }
                }
            }
            stats.total_processed += 1;
        }

        offset += batch_size;
    }

    info!("ETL Complete: processed={}, basic={}, social={}, technical={}",
          stats.total_processed, stats.basic_updated, stats.social_updated, stats.technical_updated);

    Ok(())
}


#[derive(QueryableByName, Debug)]
struct MetadataRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    sid: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    symbol: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    market_cap_rank: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    json_data: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    last_updated: DateTime<Utc>,
}

fn process_overview_basic(
    conn: &mut PgConnection,
    row: &MetadataRow,
    data: &Value,
) -> Result<()> {
    let market_data = data.get("market_data");

    // Extract description from the 'description' object
    let description = data
        .get("description")
        .and_then(|d| d.get("en"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())  // Filter out empty strings
        .map(|s| s.to_string());



    let current_price = market_data
        .and_then(|md| md.get("current_price"))
        .and_then(|cp| cp.get("usd"))
        .and_then(|v| v.as_f64())
        .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

    let market_cap = market_data
        .and_then(|md| md.get("market_cap"))
        .and_then(|mc| mc.get("usd"))
        .and_then(|v| v.as_f64())
        .map(|v| v as i64);

    let volume_24h = market_data
        .and_then(|md| md.get("total_volume"))
        .and_then(|tv| tv.get("usd"))
        .and_then(|v| v.as_f64())
        .map(|v| v as i64);

    // Extract supply data
    let circulating_supply = market_data
        .and_then(|md| md.get("circulating_supply"))
        .and_then(|v| v.as_f64())
        .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

    let total_supply = market_data
        .and_then(|md| md.get("total_supply"))
        .and_then(|v| v.as_f64())
        .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

    let max_supply = market_data
        .and_then(|md| md.get("max_supply"))
        .and_then(|v| v.as_f64())
        .and_then(|v| BigDecimal::from_str(&v.to_string()).ok());

    // Generate slug from symbol
    let slug = row.symbol.to_lowercase().replace(' ', "-");

    diesel::sql_query(
        "INSERT INTO crypto_overview_basic
         (sid, symbol, name, slug, description, market_cap_rank, market_cap,
          volume_24h, current_price, circulating_supply, total_supply, max_supply, last_updated)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         ON CONFLICT (sid) DO UPDATE SET
            slug = EXCLUDED.slug,
            description = COALESCE(EXCLUDED.description, crypto_overview_basic.description),
            market_cap_rank = COALESCE(EXCLUDED.market_cap_rank, crypto_overview_basic.market_cap_rank),
            market_cap = COALESCE(EXCLUDED.market_cap, crypto_overview_basic.market_cap),
            volume_24h = COALESCE(EXCLUDED.volume_24h, crypto_overview_basic.volume_24h),
            current_price = COALESCE(EXCLUDED.current_price, crypto_overview_basic.current_price),
            circulating_supply = COALESCE(EXCLUDED.circulating_supply, crypto_overview_basic.circulating_supply),
            total_supply = COALESCE(EXCLUDED.total_supply, crypto_overview_basic.total_supply),
            max_supply = COALESCE(EXCLUDED.max_supply, crypto_overview_basic.max_supply),
            last_updated = EXCLUDED.last_updated"
    )
        .bind::<diesel::sql_types::BigInt, _>(row.sid)
        .bind::<diesel::sql_types::Text, _>(&row.symbol)
        .bind::<diesel::sql_types::Text, _>(&row.name)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(Some(&slug))
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(description.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(row.market_cap_rank)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::BigInt>, _>(market_cap)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::BigInt>, _>(volume_24h)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(current_price.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(circulating_supply.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(total_supply.as_ref())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(max_supply.as_ref())
        .bind::<diesel::sql_types::Timestamptz, _>(row.last_updated)
        .execute(conn)?;

    Ok(())
}

fn process_social(
    conn: &mut PgConnection,
    sid: i64,
    data: &Value,
) -> Result<()> {
    let links = data.get("links");


    let website_url = links
        .and_then(|l| l.get("homepage"))
        .and_then(|h| h.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let twitter_handle = links
        .and_then(|l| l.get("twitter_screen_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Get blockchain_site (not blockchain_sites)
    let blockchain_sites = links
        .and_then(|l| l.get("blockchain_site"))  // Make sure this matches the JSON key
        .cloned();

    // Only update if we have at least one non-null value
    if website_url.is_some() || twitter_handle.is_some() || blockchain_sites.is_some() {
        diesel::sql_query(
            "INSERT INTO crypto_social (sid, website_url, twitter_handle, blockchain_sites)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (sid) DO UPDATE SET
                website_url = COALESCE(EXCLUDED.website_url, crypto_social.website_url),
                twitter_handle = COALESCE(EXCLUDED.twitter_handle, crypto_social.twitter_handle),
                blockchain_sites = COALESCE(EXCLUDED.blockchain_sites, crypto_social.blockchain_sites)"
        )
            .bind::<diesel::sql_types::BigInt, _>(sid)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(website_url)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(twitter_handle)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Jsonb>, _>(blockchain_sites.as_ref())
            .execute(conn)?;
    }

    Ok(())
}

fn process_technical(
    conn: &mut PgConnection,
    sid: i64,
    data: &Value,
) -> Result<()> {
    let categories = data
        .get("categories")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_lowercase())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let is_defi = categories.iter().any(|c| c.contains("defi"));
    let is_stablecoin = categories.iter().any(|c| c.contains("stablecoin"));

    diesel::sql_query(
        "INSERT INTO crypto_technical (sid, is_defi, is_stablecoin)
         VALUES ($1, $2, $3)
         ON CONFLICT (sid) DO UPDATE SET
            is_defi = EXCLUDED.is_defi,
            is_stablecoin = EXCLUDED.is_stablecoin"
    )
        .bind::<diesel::sql_types::BigInt, _>(sid)
        .bind::<diesel::sql_types::Bool, _>(is_defi)
        .bind::<diesel::sql_types::Bool, _>(is_stablecoin)
        .execute(conn)?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub total_processed: usize,
    pub basic_updated: usize,
    pub social_updated: usize,
    pub technical_updated: usize,
}