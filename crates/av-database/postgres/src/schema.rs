// @generated automatically by Diesel CLI.

diesel::table! {
    authormaps (id) {
        id -> Int4,
        feedid -> Int4,
        authorid -> Int4,
    }
}

diesel::table! {
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
    newsoverviews (creation, id) {
        id -> Int4,
        creation -> Timestamptz,
        sid -> Int8,
        items -> Int4,
        hashid -> Text,
    }
}

diesel::table! {
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
        mod_time -> Timestamptz,
    }
}

diesel::table! {
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
        mod_time -> Timestamptz,
    }
}

diesel::table! {
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
    topicmaps (id) {
        id -> Int4,
        sid -> Int8,
        feedid -> Int4,
        topicid -> Int4,
        relscore -> Float4,
    }
}

diesel::table! {
    topicrefs (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
    }
}

diesel::table! {
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

diesel::joinable!(authormaps -> feeds (feedid));
diesel::joinable!(feeds -> symbols (sid));
diesel::joinable!(intradayprices -> symbols (sid));
diesel::joinable!(newsoverviews -> symbols (sid));
diesel::joinable!(overviewexts -> symbols (sid));
diesel::joinable!(overviews -> symbols (sid));
diesel::joinable!(summaryprices -> symbols (sid));
diesel::joinable!(tickersentiments -> feeds (feedid));
diesel::joinable!(tickersentiments -> symbols (sid));
diesel::joinable!(topicmaps -> feeds (feedid));
diesel::joinable!(topicmaps -> symbols (sid));
diesel::joinable!(topicmaps -> topicrefs (topicid));
diesel::joinable!(topstats -> symbols (sid));

diesel::allow_tables_to_appear_in_same_query!(
    authormaps,
    feeds,
    intradayprices,
    newsoverviews,
    overviewexts,
    overviews,
    summaryprices,
    symbols,
    tickersentiments,
    topicmaps,
    topicrefs,
    topstats,
);
