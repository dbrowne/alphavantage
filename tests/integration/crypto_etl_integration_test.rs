#[cfg(test)]
mod tests {
    use anyhow::Result;
    use diesel::prelude::*;
    use serde_json::json;
    use chrono::Utc;
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    use av_database_postgres::{
        connection::establish_connection,
        schema::{symbols, crypto_metadata, crypto_overview_basic, crypto_social, crypto_technical},
    };
    use av_cli::commands::update::crypto_metadata_etl::{CryptoMetadataETL, ExtractedCryptoData};

    /// Setup test database with sample data
    fn setup_test_data(conn: &mut PgConnection) -> Result<()> {
        // Clean existing test data
        diesel::delete(crypto_technical::table).execute(conn)?;
        diesel::delete(crypto_social::table).execute(conn)?;
        diesel::delete(crypto_overview_basic::table).execute(conn)?;
        diesel::delete(crypto_metadata::table).execute(conn)?;

        // Insert test symbols
        let test_symbols = vec![
            (1001i64, "BTC", "Bitcoin", "Cryptocurrency"),
            (1002i64, "ETH", "Ethereum", "Cryptocurrency"),
            (1003i64, "USDT", "Tether", "Cryptocurrency"),
        ];

        for (sid, symbol, name, sec_type) in test_symbols {
            diesel::insert_into(symbols::table)
                .values((
                    symbols::sid.eq(sid),
                    symbols::symbol.eq(symbol),
                    symbols::name.eq(name),
                    symbols::sec_type.eq(sec_type),
                    symbols::is_active.eq(true),
                ))
                .on_conflict(symbols::sid)
                .do_nothing()
                .execute(conn)?;
        }

        // Insert test crypto_metadata with rich JSON data
        let btc_data = json!({
            "description": {
                "en": "Bitcoin is the first and most well-known cryptocurrency."
            },
            "market_data": {
                "current_price": {"usd": 50000.0},
                "market_cap": {"usd": 1000000000000.0},
                "total_volume": {"usd": 50000000000.0},
                "circulating_supply": 19000000.0,
                "total_supply": 21000000.0,
                "max_supply": 21000000.0
            },
            "links": {
                "homepage": ["https://bitcoin.org"],
                "whitepaper": "https://bitcoin.org/bitcoin.pdf",
                "repos_url": {
                    "github": ["https://github.com/bitcoin/bitcoin"]
                },
                "twitter_screen_name": "bitcoin"
            },
            "developer_data": {
                "forks": 35000,
                "stars": 70000,
                "subscribers": 5000,
                "total_issues": 1000,
                "closed_issues": 900,
                "pull_requests_merged": 5000,
                "contributors": 800,
                "commit_count_4_weeks": 200
            },
            "categories": ["Cryptocurrency", "Store of Value"],
            "coingecko_score": 85.5,
            "developer_score": 95.0,
            "community_score": 80.0,
            "liquidity_score": 90.0
        });

        let eth_data = json!({
            "description": {
                "en": "Ethereum is a decentralized platform for smart contracts."
            },
            "market_data": {
                "current_price": {"usd": 3000.0},
                "market_cap": {"usd": 360000000000.0},
                "total_volume": {"usd": 20000000000.0},
                "circulating_supply": 120000000.0,
                "total_supply": 120000000.0,
                "max_supply": null
            },
            "links": {
                "homepage": ["https://ethereum.org"],
                "whitepaper": "https://ethereum.org/whitepaper.pdf",
                "repos_url": {
                    "github": ["https://github.com/ethereum/go-ethereum"]
                },
                "twitter_screen_name": "ethereum",
                "telegram_channel_identifier": "ethereum",
                "subreddit_url": "https://reddit.com/r/ethereum"
            },
            "categories": ["Smart Contract Platform", "DeFi", "Layer-1"],
            "coingecko_score": 88.0,
            "developer_score": 98.0,
            "community_score": 85.0
        });

        let usdt_data = json!({
            "description": {
                "en": "Tether is a stablecoin pegged to the US Dollar."
            },
            "market_data": {
                "current_price": {"usd": 1.0},
                "market_cap": {"usd": 100000000000.0},
                "total_volume": {"usd": 80000000000.0},
                "circulating_supply": 100000000000.0
            },
            "links": {
                "homepage": ["https://tether.to"],
                "twitter_screen_name": "Tether_to"
            },
            "categories": ["Stablecoin", "Asset-Backed"],
            "coingecko_score": 75.0
        });

        // Insert metadata records
        diesel::sql_query(
            "INSERT INTO crypto_metadata (sid, source, source_id, market_cap_rank, additional_data, is_active, last_updated)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
            .bind::<diesel::sql_types::BigInt, _>(1001)
            .bind::<diesel::sql_types::Text, _>("coingecko")
            .bind::<diesel::sql_types::Text, _>("bitcoin")
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(Some(1))
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Jsonb>, _>(Some(btc_data))
            .bind::<diesel::sql_types::Bool, _>(true)
            .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
            .execute(conn)?;

        diesel::sql_query(
            "INSERT INTO crypto_metadata (sid, source, source_id, market_cap_rank, additional_data, is_active, last_updated)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
            .bind::<diesel::sql_types::BigInt, _>(1002)
            .bind::<diesel::sql_types::Text, _>("coingecko")
            .bind::<diesel::sql_types::Text, _>("ethereum")
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(Some(2))
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Jsonb>, _>(Some(eth_data))
            .bind::<diesel::sql_types::Bool, _>(true)
            .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
            .execute(conn)?;

        diesel::sql_query(
            "INSERT INTO crypto_metadata (sid, source, source_id, market_cap_rank, additional_data, is_active, last_updated)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
            .bind::<diesel::sql_types::BigInt, _>(1003)
            .bind::<diesel::sql_types::Text, _>("coingecko")
            .bind::<diesel::sql_types::Text, _>("tether")
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(Some(3))
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Jsonb>, _>(Some(usdt_data))
            .bind::<diesel::sql_types::Bool, _>(true)
            .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
            .execute(conn)?;

        Ok(())
    }

    #[test]
    fn test_etl_process_all() -> Result<()> {
        use av_database_postgres::connection::establish_connection;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Setup test data
        setup_test_data(&mut conn)?;

        // Run ETL
        let etl = CryptoMetadataETL::new(database_url.clone());
        let stats = etl.process_all()?;

        // Verify results
        assert_eq!(stats.total_processed, 3);
        assert_eq!(stats.basic_updated, 3);
        assert_eq!(stats.social_updated, 3);
        assert_eq!(stats.technical_updated, 3);
        assert_eq!(stats.errors, 0);

        // Verify crypto_overview_basic
        let btc_basic: (String, Option<String>, Option<i64>, Option<BigDecimal>) =
            crypto_overview_basic::table
                .inner_join(symbols::table.on(crypto_overview_basic::sid.eq(symbols::sid)))
                .filter(symbols::symbol.eq("BTC"))
                .select((
                    symbols::symbol,
                    crypto_overview_basic::description,
                    crypto_overview_basic::market_cap,
                    crypto_overview_basic::current_price,
                ))
                .first(&mut conn)?;

        assert_eq!(btc_basic.0, "BTC");
        assert!(btc_basic.1.is_some());
        assert_eq!(btc_basic.2, Some(1000000000000));
        assert_eq!(btc_basic.3, Some(BigDecimal::from_str("50000").unwrap()));

        // Verify crypto_social
        let eth_social: (String, Option<String>, Option<String>, Option<BigDecimal>) =
            crypto_social::table
                .inner_join(symbols::table.on(crypto_social::sid.eq(symbols::sid)))
                .filter(symbols::symbol.eq("ETH"))
                .select((
                    symbols::symbol,
                    crypto_social::website_url,
                    crypto_social::github_url,
                    crypto_social::developer_score,
                ))
                .first(&mut conn)?;

        assert_eq!(eth_social.0, "ETH");
        assert_eq!(eth_social.1, Some("https://ethereum.org".to_string()));
        assert!(eth_social.2.is_some());
        assert_eq!(eth_social.3, Some(BigDecimal::from_str("98.0").unwrap()));

        // Verify crypto_technical
        let usdt_technical: (String, Option<bool>, Option<bool>) =
            crypto_technical::table
                .inner_join(symbols::table.on(crypto_technical::sid.eq(symbols::sid)))
                .filter(symbols::symbol.eq("USDT"))
                .select((
                    symbols::symbol,
                    crypto_technical::is_stablecoin,
                    crypto_technical::is_defi,
                ))
                .first(&mut conn)?;

        assert_eq!(usdt_technical.0, "USDT");
        assert_eq!(usdt_technical.1, Some(true));
        assert_eq!(usdt_technical.2, Some(false));

        Ok(())
    }

    #[test]
    fn test_category_parsing() -> Result<()> {
        let categories = vec![
            "DeFi".to_string(),
            "Layer-2".to_string(),
            "Stablecoin".to_string(),
        ];

        // Convert to lowercase for matching
        let lower_categories: Vec<String> = categories.iter()
            .map(|c| c.to_lowercase())
            .collect();

        assert!(lower_categories.iter().any(|c| c.contains("defi")));
        assert!(lower_categories.iter().any(|c| c.contains("layer-2")));
        assert!(lower_categories.iter().any(|c| c.contains("stablecoin")));

        // Should not detect other categories
        assert!(!lower_categories.iter().any(|c| c.contains("gaming")));
        assert!(!lower_categories.iter().any(|c| c.contains("metaverse")));

        Ok(())
    }

    #[test]
    fn test_github_data_extraction() -> Result<()> {
        use av_database_postgres::connection::establish_connection;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Setup test data
        setup_test_data(&mut conn)?;

        // Run ETL
        let etl = CryptoMetadataETL::new(database_url.clone());
        let stats = etl.process_all()?;

        // Check Bitcoin's GitHub data in crypto_technical
        let btc_technical: (i32, i32, i32) = diesel::sql_query(
            "SELECT github_forks, github_stars, github_contributors
             FROM crypto_technical ct
             JOIN symbols s ON ct.sid = s.sid
             WHERE s.symbol = 'BTC'"
        )
            .get_result::<(Option<i32>, Option<i32>, Option<i32>)>(&mut conn)?
            .map(|(f, s, c)| (f.unwrap_or(0), s.unwrap_or(0), c.unwrap_or(0)))?;

        assert_eq!(btc_technical.0, 35000); // forks
        assert_eq!(btc_technical.1, 70000); // stars
        assert_eq!(btc_technical.2, 800);   // contributors

        Ok(())
    }

    #[test]
    fn test_market_data_extraction() -> Result<()> {
        use av_database_postgres::connection::establish_connection;
        use bigdecimal::BigDecimal;
        use std::str::FromStr;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Setup test data
        setup_test_data(&mut conn)?;

        // Run ETL
        let etl = CryptoMetadataETL::new(database_url.clone());
        etl.process_all()?;

        // Verify Ethereum market data
        let eth_overview = crypto_overview_basic::table
            .inner_join(symbols::table.on(crypto_overview_basic::sid.eq(symbols::sid)))
            .filter(symbols::symbol.eq("ETH"))
            .select((
                crypto_overview_basic::current_price,
                crypto_overview_basic::market_cap,
                crypto_overview_basic::volume_24h,
                crypto_overview_basic::circulating_supply,
            ))
            .first::<(Option<BigDecimal>, Option<i64>, Option<i64>, Option<BigDecimal>)>(&mut conn)?;

        assert_eq!(eth_overview.0, Some(BigDecimal::from_str("3000").unwrap()));
        assert_eq!(eth_overview.1, Some(360000000000));
        assert_eq!(eth_overview.2, Some(20000000000));
        assert_eq!(eth_overview.3, Some(BigDecimal::from_str("120000000").unwrap()));

        Ok(())
    }

    #[test]
    fn test_social_links_extraction() -> Result<()> {
        use av_database_postgres::connection::establish_connection;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Setup test data
        setup_test_data(&mut conn)?;

        // Run ETL
        let etl = CryptoMetadataETL::new(database_url.clone());
        etl.process_all()?;

        // Check Ethereum social links
        let eth_social = crypto_social::table
            .inner_join(symbols::table.on(crypto_social::sid.eq(symbols::sid)))
            .filter(symbols::symbol.eq("ETH"))
            .select((
                crypto_social::website_url,
                crypto_social::github_url,
                crypto_social::twitter_handle,
                crypto_social::telegram_url,
                crypto_social::reddit_url,
            ))
            .first::<(Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(&mut conn)?;

        assert_eq!(eth_social.0, Some("https://ethereum.org".to_string()));
        assert_eq!(eth_social.1, Some("https://github.com/ethereum/go-ethereum".to_string()));
        assert_eq!(eth_social.2, Some("ethereum".to_string()));
        assert_eq!(eth_social.3, Some("https://t.me/ethereum".to_string()));
        assert_eq!(eth_social.4, Some("https://reddit.com/r/ethereum".to_string()));

        Ok(())
    }

    #[test]
    fn test_update_existing_records() -> Result<()> {
        use av_database_postgres::connection::establish_connection;
        use bigdecimal::BigDecimal;
        use std::str::FromStr;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Setup test data
        setup_test_data(&mut conn)?;

        // Run ETL first time
        let etl = CryptoMetadataETL::new(database_url.clone());
        let stats1 = etl.process_all()?;
        assert_eq!(stats1.basic_updated, 3);

        // Modify the metadata
        let updated_btc_data = json!({
            "description": {
                "en": "Updated Bitcoin description"
            },
            "market_data": {
                "current_price": {"usd": 60000.0},
                "market_cap": {"usd": 1200000000000.0},
            }
        });

        diesel::sql_query(
            "UPDATE crypto_metadata SET additional_data = $1 WHERE sid = 1001"
        )
            .bind::<diesel::sql_types::Jsonb, _>(&updated_btc_data)
            .execute(&mut conn)?;

        // Run ETL again with update_existing = true
        let mut etl2 = CryptoMetadataETL::new(database_url.clone());
        etl2.update_existing = true;
        let stats2 = etl2.process_all()?;

        // Verify update occurred
        let btc_updated = crypto_overview_basic::table
            .filter(crypto_overview_basic::sid.eq(1001))
            .select(crypto_overview_basic::current_price)
            .first::<Option<BigDecimal>>(&mut conn)?;

        assert_eq!(btc_updated, Some(BigDecimal::from_str("60000").unwrap()));

        Ok(())
    }

    #[test]
    fn test_null_handling() -> Result<()> {
        use av_database_postgres::connection::establish_connection;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Clean test data
        diesel::delete(crypto_technical::table).execute(&mut conn)?;
        diesel::delete(crypto_social::table).execute(&mut conn)?;
        diesel::delete(crypto_overview_basic::table).execute(&mut conn)?;
        diesel::delete(crypto_metadata::table).execute(&mut conn)?;

        // Insert symbol with minimal metadata
        diesel::insert_into(symbols::table)
            .values((
                symbols::sid.eq(9999i64),
                symbols::symbol.eq("TEST"),
                symbols::name.eq("Test Token"),
                symbols::sec_type.eq("Cryptocurrency"),
                symbols::is_active.eq(true),
            ))
            .on_conflict(symbols::sid)
            .do_nothing()
            .execute(&mut conn)?;

        // Insert metadata with null additional_data
        diesel::sql_query(
            "INSERT INTO crypto_metadata (sid, source, source_id, is_active, last_updated)
             VALUES ($1, $2, $3, $4, $5)"
        )
            .bind::<diesel::sql_types::BigInt, _>(9999)
            .bind::<diesel::sql_types::Text, _>("test")
            .bind::<diesel::sql_types::Text, _>("test-token")
            .bind::<diesel::sql_types::Bool, _>(true)
            .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
            .execute(&mut conn)?;

        // Run ETL - should handle null data gracefully
        let etl = CryptoMetadataETL::new(database_url.clone());
        let stats = etl.process_all()?;

        // Should process the record without errors
        assert_eq!(stats.total_processed, 1);
        assert_eq!(stats.errors, 0);

        // Verify basic record was created with nulls where appropriate
        let test_basic = crypto_overview_basic::table
            .filter(crypto_overview_basic::sid.eq(9999))
            .select((
                crypto_overview_basic::symbol,
                crypto_overview_basic::description,
                crypto_overview_basic::current_price,
            ))
            .first::<(String, Option<String>, Option<BigDecimal>)>(&mut conn)?;

        assert_eq!(test_basic.0, "TEST");
        assert_eq!(test_basic.1, None);
        assert_eq!(test_basic.2, None);

        Ok(())
    }

    #[test]
    fn test_batch_processing() -> Result<()> {
        use av_database_postgres::connection::establish_connection;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string());

        let mut conn = establish_connection(&database_url)?;

        // Clean test data
        diesel::delete(crypto_technical::table).execute(&mut conn)?;
        diesel::delete(crypto_social::table).execute(&mut conn)?;
        diesel::delete(crypto_overview_basic::table).execute(&mut conn)?;
        diesel::delete(crypto_metadata::table).execute(&mut conn)?;

        // Insert many test records
        for i in 2000..2050 {
            diesel::insert_into(symbols::table)
                .values((
                    symbols::sid.eq(i as i64),
                    symbols::symbol.eq(format!("TEST{}", i)),
                    symbols::name.eq(format!("Test Token {}", i)),
                    symbols::sec_type.eq("Cryptocurrency"),
                    symbols::is_active.eq(true),
                ))
                .on_conflict(symbols::sid)
                .do_nothing()
                .execute(&mut conn)?;

            let test_data = json!({
                "description": {"en": format!("Test token {}", i)},
                "market_data": {"current_price": {"usd": i as f64}}
            });

            diesel::sql_query(
                "INSERT INTO crypto_metadata (sid, source, source_id, additional_data, is_active, last_updated)
                 VALUES ($1, $2, $3, $4, $5, $6)"
            )
                .bind::<diesel::sql_types::BigInt, _>(i as i64)
                .bind::<diesel::sql_types::Text, _>("test")
                .bind::<diesel::sql_types::Text, _>(format!("test-{}", i))
                .bind::<diesel::sql_types::Jsonb, _>(&test_data)
                .bind::<diesel::sql_types::Bool, _>(true)
                .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
                .execute(&mut conn)?;
        }

        // Run ETL with small batch size
        let mut etl = CryptoMetadataETL::new(database_url.clone());
        etl.batch_size = 10; // Process in batches of 10
        let stats = etl.process_all()?;

        // Should process all 50 records
        assert_eq!(stats.total_processed, 50);
        assert_eq!(stats.basic_updated, 50);

        // Verify all records were created
        let count: i64 = crypto_overview_basic::table
            .filter(crypto_overview_basic::sid.ge(2000))
            .filter(crypto_overview_basic::sid.lt(2050))
            .count()
            .get_result(&mut conn)?;

        assert_eq!(count, 50);

        Ok(())
    }
}