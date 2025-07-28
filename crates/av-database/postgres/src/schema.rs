// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;

    articles (hashid) {
        hashid -> Text,
        sourceid -> Int4,
        category -> Text,
        title -> Text,
        url -> Text,
        summary -> Text,
        banner -> Text,
        author -> Int4,
        ct -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    authormaps (id) {
        id -> Int4,
        feedid -> Int4,
        authorid -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    authors (id) {
        id -> Int4,
        author_name -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    crypto_markets (id) {
        id -> Int4,
        sid -> Int8,
        #[max_length = 100]
        exchange -> Varchar,
        #[max_length = 20]
        base -> Varchar,
        #[max_length = 20]
        target -> Varchar,
        #[max_length = 20]
        market_type -> Nullable<Varchar>,
        volume_24h -> Nullable<Numeric>,
        volume_percentage -> Nullable<Numeric>,
        bid_ask_spread_pct -> Nullable<Numeric>,
        #[max_length = 20]
        liquidity_score -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        is_anomaly -> Nullable<Bool>,
        is_stale -> Nullable<Bool>,
        #[max_length = 20]
        trust_score -> Nullable<Varchar>,
        last_traded_at -> Nullable<Timestamptz>,
        last_fetch_at -> Nullable<Timestamptz>,
        c_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    crypto_overviews (sid) {
        sid -> Int8,
        #[max_length = 20]
        symbol -> Varchar,
        name -> Text,
        #[max_length = 100]
        slug -> Nullable<Varchar>,
        description -> Nullable<Text>,
        market_cap_rank -> Nullable<Int4>,
        market_cap -> Nullable<Int8>,
        fully_diluted_valuation -> Nullable<Int8>,
        volume_24h -> Nullable<Int8>,
        volume_change_24h -> Nullable<Numeric>,
        current_price -> Nullable<Numeric>,
        price_change_24h -> Nullable<Numeric>,
        price_change_pct_24h -> Nullable<Numeric>,
        price_change_pct_7d -> Nullable<Numeric>,
        price_change_pct_14d -> Nullable<Numeric>,
        price_change_pct_30d -> Nullable<Numeric>,
        price_change_pct_60d -> Nullable<Numeric>,
        price_change_pct_200d -> Nullable<Numeric>,
        price_change_pct_1y -> Nullable<Numeric>,
        ath -> Nullable<Numeric>,
        ath_date -> Nullable<Timestamptz>,
        ath_change_percentage -> Nullable<Numeric>,
        atl -> Nullable<Numeric>,
        atl_date -> Nullable<Timestamptz>,
        atl_change_percentage -> Nullable<Numeric>,
        roi_times -> Nullable<Numeric>,
        #[max_length = 10]
        roi_currency -> Nullable<Varchar>,
        roi_percentage -> Nullable<Numeric>,
        circulating_supply -> Nullable<Numeric>,
        total_supply -> Nullable<Numeric>,
        max_supply -> Nullable<Numeric>,
        last_updated -> Nullable<Timestamptz>,
        c_time -> Timestamptz,
        m_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    crypto_social (sid) {
        sid -> Int8,
        website_url -> Nullable<Text>,
        whitepaper_url -> Nullable<Text>,
        github_url -> Nullable<Text>,
        #[max_length = 100]
        twitter_handle -> Nullable<Varchar>,
        twitter_followers -> Nullable<Int4>,
        telegram_url -> Nullable<Text>,
        telegram_members -> Nullable<Int4>,
        discord_url -> Nullable<Text>,
        discord_members -> Nullable<Int4>,
        reddit_url -> Nullable<Text>,
        reddit_subscribers -> Nullable<Int4>,
        facebook_url -> Nullable<Text>,
        facebook_likes -> Nullable<Int4>,
        coingecko_score -> Nullable<Numeric>,
        developer_score -> Nullable<Numeric>,
        community_score -> Nullable<Numeric>,
        liquidity_score -> Nullable<Numeric>,
        public_interest_score -> Nullable<Numeric>,
        sentiment_votes_up_pct -> Nullable<Numeric>,
        sentiment_votes_down_pct -> Nullable<Numeric>,
        c_time -> Timestamptz,
        m_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    crypto_technical (sid) {
        sid -> Int8,
        #[max_length = 100]
        blockchain_platform -> Nullable<Varchar>,
        #[max_length = 50]
        token_standard -> Nullable<Varchar>,
        #[max_length = 100]
        consensus_mechanism -> Nullable<Varchar>,
        #[max_length = 100]
        hashing_algorithm -> Nullable<Varchar>,
        block_time_minutes -> Nullable<Numeric>,
        block_reward -> Nullable<Numeric>,
        block_height -> Nullable<Int8>,
        hash_rate -> Nullable<Numeric>,
        difficulty -> Nullable<Numeric>,
        github_forks -> Nullable<Int4>,
        github_stars -> Nullable<Int4>,
        github_subscribers -> Nullable<Int4>,
        github_total_issues -> Nullable<Int4>,
        github_closed_issues -> Nullable<Int4>,
        github_pull_requests -> Nullable<Int4>,
        github_contributors -> Nullable<Int4>,
        github_commits_4_weeks -> Nullable<Int4>,
        is_defi -> Nullable<Bool>,
        is_stablecoin -> Nullable<Bool>,
        is_nft_platform -> Nullable<Bool>,
        is_exchange_token -> Nullable<Bool>,
        is_gaming -> Nullable<Bool>,
        is_metaverse -> Nullable<Bool>,
        is_privacy_coin -> Nullable<Bool>,
        is_layer2 -> Nullable<Bool>,
        is_wrapped -> Nullable<Bool>,
        genesis_date -> Nullable<Date>,
        ico_price -> Nullable<Numeric>,
        ico_date -> Nullable<Date>,
        c_time -> Timestamptz,
        m_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    feeds (id) {
        id -> Int4,
        sid -> Int8,
        newsoverviewid -> Int4,
        articleid -> Text,
        sourceid -> Int4,
        osentiment -> Float4,
        #[max_length = 20]
        sentlabel -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    intradayprices (tstamp, sid, eventid) {
        eventid -> Int8,
        tstamp -> Timestamptz,
        sid -> Int8,
        #[max_length = 20]
        symbol -> Varchar,
        open -> Float4,
        high -> Float4,
        low -> Float4,
        close -> Float4,
        volume -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    newsoverviews (creation, id) {
        id -> Int4,
        creation -> Timestamptz,
        sid -> Int8,
        items -> Int4,
        hashid -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    overviewexts (sid) {
        sid -> Int8,
        revenue_per_share_ttm -> Float4,
        profit_margin -> Float4,
        operating_margin_ttm -> Float4,
        return_on_assets_ttm -> Float4,
        return_on_equity_ttm -> Float4,
        revenue_ttm -> Int8,
        gross_profit_ttm -> Int8,
        diluted_eps_ttm -> Float4,
        quarterly_earnings_growth_yoy -> Float4,
        quarterly_revenue_growth_yoy -> Float4,
        analyst_target_price -> Float4,
        trailing_pe -> Float4,
        forward_pe -> Float4,
        price_to_sales_ratio_ttm -> Float4,
        price_to_book_ratio -> Float4,
        ev_to_revenue -> Float4,
        ev_to_ebitda -> Float4,
        beta -> Float4,
        week_high_52 -> Float4,
        week_low_52 -> Float4,
        day_moving_average_50 -> Float4,
        day_moving_average_200 -> Float4,
        shares_outstanding -> Int8,
        dividend_date -> Nullable<Date>,
        ex_dividend_date -> Nullable<Date>,
        c_time -> Timestamptz,
        m_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    overviews (sid) {
        sid -> Int8,
        #[max_length = 20]
        symbol -> Varchar,
        name -> Text,
        description -> Text,
        #[max_length = 20]
        cik -> Varchar,
        #[max_length = 20]
        exchange -> Varchar,
        #[max_length = 10]
        currency -> Varchar,
        #[max_length = 50]
        country -> Varchar,
        #[max_length = 100]
        sector -> Varchar,
        #[max_length = 100]
        industry -> Varchar,
        address -> Text,
        #[max_length = 20]
        fiscal_year_end -> Varchar,
        latest_quarter -> Date,
        market_capitalization -> Int8,
        ebitda -> Int8,
        pe_ratio -> Float4,
        peg_ratio -> Float4,
        book_value -> Float4,
        dividend_per_share -> Float4,
        dividend_yield -> Float4,
        eps -> Float4,
        c_time -> Timestamptz,
        m_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    procstates (spid) {
        spid -> Int4,
        proc_id -> Nullable<Int4>,
        start_time -> Timestamp,
        end_state -> Nullable<Int4>,
        end_time -> Nullable<Timestamp>,
        error_msg -> Nullable<Text>,
        records_processed -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    proctypes (id) {
        id -> Int4,
        name -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    sources (id) {
        id -> Int4,
        source_name -> Text,
        domain -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    states (id) {
        id -> Int4,
        name -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    summaryprices (tstamp, sid, eventid) {
        eventid -> Int8,
        tstamp -> Timestamptz,
        date -> Date,
        sid -> Int8,
        #[max_length = 20]
        symbol -> Varchar,
        open -> Float4,
        high -> Float4,
        low -> Float4,
        close -> Float4,
        volume -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    symbols (sid) {
        sid -> Int8,
        #[max_length = 20]
        symbol -> Varchar,
        name -> Text,
        #[max_length = 50]
        sec_type -> Varchar,
        #[max_length = 10]
        region -> Varchar,
        market_open -> Time,
        market_close -> Time,
        #[max_length = 50]
        timezone -> Varchar,
        #[max_length = 10]
        currency -> Varchar,
        overview -> Bool,
        intraday -> Bool,
        summary -> Bool,
        c_time -> Timestamptz,
        m_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    tickersentiments (id) {
        id -> Int4,
        feedid -> Int4,
        sid -> Int8,
        relevance -> Float4,
        tsentiment -> Float4,
        #[max_length = 20]
        sentiment_label -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    topicmaps (id) {
        id -> Int4,
        sid -> Int8,
        feedid -> Int4,
        topicid -> Int4,
        relscore -> Float4,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    topicrefs (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;

    topstats (date, event_type, sid) {
        date -> Timestamptz,
        #[max_length = 50]
        event_type -> Varchar,
        sid -> Int8,
        #[max_length = 20]
        symbol -> Varchar,
        price -> Float4,
        change_val -> Float4,
        change_pct -> Float4,
        volume -> Int8,
    }
}

diesel::joinable!(articles -> authors (author));
diesel::joinable!(articles -> sources (sourceid));
diesel::joinable!(authormaps -> feeds (feedid));
diesel::joinable!(crypto_markets -> symbols (sid));
diesel::joinable!(crypto_overviews -> symbols (sid));
diesel::joinable!(crypto_social -> symbols (sid));
diesel::joinable!(crypto_technical -> symbols (sid));
diesel::joinable!(feeds -> symbols (sid));
diesel::joinable!(intradayprices -> symbols (sid));
diesel::joinable!(newsoverviews -> symbols (sid));
diesel::joinable!(overviewexts -> symbols (sid));
diesel::joinable!(overviews -> symbols (sid));
diesel::joinable!(procstates -> proctypes (proc_id));
diesel::joinable!(procstates -> states (end_state));
diesel::joinable!(summaryprices -> symbols (sid));
diesel::joinable!(tickersentiments -> feeds (feedid));
diesel::joinable!(tickersentiments -> symbols (sid));
diesel::joinable!(topicmaps -> feeds (feedid));
diesel::joinable!(topicmaps -> symbols (sid));
diesel::joinable!(topicmaps -> topicrefs (topicid));
diesel::joinable!(topstats -> symbols (sid));

diesel::allow_tables_to_appear_in_same_query!(
    articles,
    authormaps,
    authors,
    crypto_markets,
    crypto_overviews,
    crypto_social,
    crypto_technical,
    feeds,
    intradayprices,
    newsoverviews,
    overviewexts,
    overviews,
    procstates,
    proctypes,
    sources,
    states,
    summaryprices,
    symbols,
    tickersentiments,
    topicmaps,
    topicrefs,
    topstats,
);
