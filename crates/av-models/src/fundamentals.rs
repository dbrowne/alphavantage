//! Fundamental analysis data models for company financials

use serde::{Deserialize, Serialize};

/// Company overview with key financial metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanyOverview {
    /// Stock symbol
    #[serde(rename = "Symbol")]
    pub symbol: String,
    
    /// Asset type (usually "Common Stock")
    #[serde(rename = "AssetType")]
    pub asset_type: String,
    
    /// Company name
    #[serde(rename = "Name")]
    pub name: String,
    
    /// Company description
    #[serde(rename = "Description")]
    pub description: String,
    
    /// Central Index Key (CIK)
    #[serde(rename = "CIK")]
    pub cik: String,
    
    /// Exchange where the stock is traded
    #[serde(rename = "Exchange")]
    pub exchange: String,
    
    /// Currency
    #[serde(rename = "Currency")]
    pub currency: String,
    
    /// Country
    #[serde(rename = "Country")]
    pub country: String,
    
    /// Business sector
    #[serde(rename = "Sector")]
    pub sector: String,
    
    /// Industry classification
    #[serde(rename = "Industry")]
    pub industry: String,
    
    /// Company address
    #[serde(rename = "Address")]
    pub address: String,
    
    /// Fiscal year end month
    #[serde(rename = "FiscalYearEnd")]
    pub fiscal_year_end: String,
    
    /// Latest quarter end date
    #[serde(rename = "LatestQuarter")]
    pub latest_quarter: String,
    
    /// Market capitalization
    #[serde(rename = "MarketCapitalization")]
    pub market_capitalization: String,
    
    /// Enterprise to Book Ratio
    #[serde(rename = "EBITDA")]
    pub ebitda: String,
    
    /// Price to Earnings ratio
    #[serde(rename = "PERatio")]
    pub pe_ratio: String,
    
    /// Price to Earnings to Growth ratio
    #[serde(rename = "PEGRatio")]
    pub peg_ratio: String,
    
    /// Book value per share
    #[serde(rename = "BookValue")]
    pub book_value: String,
    
    /// Dividend per share
    #[serde(rename = "DividendPerShare")]
    pub dividend_per_share: String,
    
    /// Dividend yield
    #[serde(rename = "DividendYield")]
    pub dividend_yield: String,
    
    /// Earnings per share
    #[serde(rename = "EPS")]
    pub eps: String,
    
    /// Revenue per share (TTM)
    #[serde(rename = "RevenuePerShareTTM")]
    pub revenue_per_share_ttm: String,
    
    /// Profit margin
    #[serde(rename = "ProfitMargin")]
    pub profit_margin: String,
    
    /// Operating margin (TTM)
    #[serde(rename = "OperatingMarginTTM")]
    pub operating_margin_ttm: String,
    
    /// Return on assets (TTM)
    #[serde(rename = "ReturnOnAssetsTTM")]
    pub return_on_assets_ttm: String,
    
    /// Return on equity (TTM)
    #[serde(rename = "ReturnOnEquityTTM")]
    pub return_on_equity_ttm: String,
    
    /// Revenue (TTM)
    #[serde(rename = "RevenueTTM")]
    pub revenue_ttm: String,
    
    /// Gross profit (TTM)
    #[serde(rename = "GrossProfitTTM")]
    pub gross_profit_ttm: String,
    
    /// Diluted EPS (TTM)
    #[serde(rename = "DilutedEPSTTM")]
    pub diluted_eps_ttm: String,
    
    /// Quarterly earnings growth (YoY)
    #[serde(rename = "QuarterlyEarningsGrowthYOY")]
    pub quarterly_earnings_growth_yoy: String,
    
    /// Quarterly revenue growth (YoY)
    #[serde(rename = "QuarterlyRevenueGrowthYOY")]
    pub quarterly_revenue_growth_yoy: String,
    
    /// Analyst price target
    #[serde(rename = "AnalystTargetPrice")]
    pub analyst_target_price: String,
    
    /// Trailing P/E ratio
    #[serde(rename = "TrailingPE")]
    pub trailing_pe: String,
    
    /// Forward P/E ratio
    #[serde(rename = "ForwardPE")]
    pub forward_pe: String,
    
    /// Price to Sales ratio (TTM)
    #[serde(rename = "PriceToSalesRatioTTM")]
    pub price_to_sales_ratio_ttm: String,
    
    /// Price to Book ratio
    #[serde(rename = "PriceToBookRatio")]
    pub price_to_book_ratio: String,
    
    /// Enterprise Value to Revenue
    #[serde(rename = "EVToRevenue")]
    pub ev_to_revenue: String,
    
    /// Enterprise Value to EBITDA
    #[serde(rename = "EVToEBITDA")]
    pub ev_to_ebitda: String,
    
    /// Beta coefficient
    #[serde(rename = "Beta")]
    pub beta: String,
    
    /// 52-week high price
    #[serde(rename = "52WeekHigh")]
    pub week_52_high: String,
    
    /// 52-week low price
    #[serde(rename = "52WeekLow")]
    pub week_52_low: String,
    
    /// 50-day moving average
    #[serde(rename = "50DayMovingAverage")]
    pub day_50_moving_average: String,
    
    /// 200-day moving average
    #[serde(rename = "200DayMovingAverage")]
    pub day_200_moving_average: String,
    
    /// Number of shares outstanding
    #[serde(rename = "SharesOutstanding")]
    pub shares_outstanding: String,
    
    /// Dividend date
    #[serde(rename = "DividendDate")]
    pub dividend_date: String,
    
    /// Ex-dividend date
    #[serde(rename = "ExDividendDate")]
    pub ex_dividend_date: String,
}

/// Income statement data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncomeStatement {
    /// Stock symbol
    pub symbol: String,
    
    /// Annual reports
    #[serde(rename = "annualReports")]
    pub annual_reports: Vec<IncomeStatementReport>,
    
    /// Quarterly reports
    #[serde(rename = "quarterlyReports")]
    pub quarterly_reports: Vec<IncomeStatementReport>,
}

/// Individual income statement report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncomeStatementReport {
    /// Fiscal date ending
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    
    /// Reported currency
    #[serde(rename = "reportedCurrency")]
    pub reported_currency: String,
    
    /// Gross profit
    #[serde(rename = "grossProfit")]
    pub gross_profit: String,
    
    /// Total revenue
    #[serde(rename = "totalRevenue")]
    pub total_revenue: String,
    
    /// Cost of revenue
    #[serde(rename = "costOfRevenue")]
    pub cost_of_revenue: String,
    
    /// Cost of goods and services sold
    #[serde(rename = "costofGoodsAndServicesSold")]
    pub cost_of_goods_and_services_sold: String,
    
    /// Operating income
    #[serde(rename = "operatingIncome")]
    pub operating_income: String,
    
    /// Selling, general and administrative expenses
    #[serde(rename = "sellingGeneralAndAdministrative")]
    pub selling_general_and_administrative: String,
    
    /// Research and development expenses
    #[serde(rename = "researchAndDevelopment")]
    pub research_and_development: String,
    
    /// Operating expenses
    #[serde(rename = "operatingExpenses")]
    pub operating_expenses: String,
    
    /// Investment income net
    #[serde(rename = "investmentIncomeNet")]
    pub investment_income_net: String,
    
    /// Net interest income
    #[serde(rename = "netInterestIncome")]
    pub net_interest_income: String,
    
    /// Interest income
    #[serde(rename = "interestIncome")]
    pub interest_income: String,
    
    /// Interest expense
    #[serde(rename = "interestExpense")]
    pub interest_expense: String,
    
    /// Non-interest income
    #[serde(rename = "nonInterestIncome")]
    pub non_interest_income: String,
    
    /// Other non-operating income
    #[serde(rename = "otherNonOperatingIncome")]
    pub other_non_operating_income: String,
    
    /// Depreciation
    #[serde(rename = "depreciation")]
    pub depreciation: String,
    
    /// Depreciation and amortization
    #[serde(rename = "depreciationAndAmortization")]
    pub depreciation_and_amortization: String,
    
    /// Income before tax
    #[serde(rename = "incomeBeforeTax")]
    pub income_before_tax: String,
    
    /// Income tax expense
    #[serde(rename = "incomeTaxExpense")]
    pub income_tax_expense: String,
    
    /// Interest and debt expense
    #[serde(rename = "interestAndDebtExpense")]
    pub interest_and_debt_expense: String,
    
    /// Net income from continuing operations
    #[serde(rename = "netIncomeFromContinuingOperations")]
    pub net_income_from_continuing_operations: String,
    
    /// Comprehensive income net of tax
    #[serde(rename = "comprehensiveIncomeNetOfTax")]
    pub comprehensive_income_net_of_tax: String,
    
    /// EBIT
    #[serde(rename = "ebit")]
    pub ebit: String,
    
    /// EBITDA
    #[serde(rename = "ebitda")]
    pub ebitda: String,
    
    /// Net income
    #[serde(rename = "netIncome")]
    pub net_income: String,
}

/// Balance sheet data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceSheet {
    /// Stock symbol
    pub symbol: String,
    
    /// Annual reports
    #[serde(rename = "annualReports")]
    pub annual_reports: Vec<BalanceSheetReport>,
    
    /// Quarterly reports
    #[serde(rename = "quarterlyReports")]
    pub quarterly_reports: Vec<BalanceSheetReport>,
}

/// Individual balance sheet report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceSheetReport {
    /// Fiscal date ending
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    
    /// Reported currency
    #[serde(rename = "reportedCurrency")]
    pub reported_currency: String,
    
    /// Total assets
    #[serde(rename = "totalAssets")]
    pub total_assets: String,
    
    /// Total current assets
    #[serde(rename = "totalCurrentAssets")]
    pub total_current_assets: String,
    
    /// Cash and cash equivalents at carrying value
    #[serde(rename = "cashAndCashEquivalentsAtCarryingValue")]
    pub cash_and_cash_equivalents_at_carrying_value: String,
    
    /// Cash and short term investments
    #[serde(rename = "cashAndShortTermInvestments")]
    pub cash_and_short_term_investments: String,
    
    /// Inventory
    #[serde(rename = "inventory")]
    pub inventory: String,
    
    /// Current net receivables
    #[serde(rename = "currentNetReceivables")]
    pub current_net_receivables: String,
    
    /// Total non-current assets
    #[serde(rename = "totalNonCurrentAssets")]
    pub total_non_current_assets: String,
    
    /// Property, plant and equipment
    #[serde(rename = "propertyPlantEquipment")]
    pub property_plant_equipment: String,
    
    /// Accumulated depreciation amortization PPE
    #[serde(rename = "accumulatedDepreciationAmortizationPPE")]
    pub accumulated_depreciation_amortization_ppe: String,
    
    /// Intangible assets
    #[serde(rename = "intangibleAssets")]
    pub intangible_assets: String,
    
    /// Intangible assets excluding goodwill
    #[serde(rename = "intangibleAssetsExcludingGoodwill")]
    pub intangible_assets_excluding_goodwill: String,
    
    /// Goodwill
    #[serde(rename = "goodwill")]
    pub goodwill: String,
    
    /// Investments
    #[serde(rename = "investments")]
    pub investments: String,
    
    /// Long term investments
    #[serde(rename = "longTermInvestments")]
    pub long_term_investments: String,
    
    /// Short term investments
    #[serde(rename = "shortTermInvestments")]
    pub short_term_investments: String,
    
    /// Other current assets
    #[serde(rename = "otherCurrentAssets")]
    pub other_current_assets: String,
    
    /// Other non-current assets
    #[serde(rename = "otherNonCurrentAssets")]
    pub other_non_current_assets: String,
    
    /// Total liabilities
    #[serde(rename = "totalLiabilities")]
    pub total_liabilities: String,
    
    /// Total current liabilities
    #[serde(rename = "totalCurrentLiabilities")]
    pub total_current_liabilities: String,
    
    /// Current accounts payable
    #[serde(rename = "currentAccountsPayable")]
    pub current_accounts_payable: String,
    
    /// Deferred revenue
    #[serde(rename = "deferredRevenue")]
    pub deferred_revenue: String,
    
    /// Current debt
    #[serde(rename = "currentDebt")]
    pub current_debt: String,
    
    /// Short term debt
    #[serde(rename = "shortTermDebt")]
    pub short_term_debt: String,
    
    /// Total non-current liabilities
    #[serde(rename = "totalNonCurrentLiabilities")]
    pub total_non_current_liabilities: String,
    
    /// Capital lease obligations
    #[serde(rename = "capitalLeaseObligations")]
    pub capital_lease_obligations: String,
    
    /// Long term debt
    #[serde(rename = "longTermDebt")]
    pub long_term_debt: String,
    
    /// Current long term debt
    #[serde(rename = "currentLongTermDebt")]
    pub current_long_term_debt: String,
    
    /// Long term debt noncurrent
    #[serde(rename = "longTermDebtNoncurrent")]
    pub long_term_debt_noncurrent: String,
    
    /// Short long term debt total
    #[serde(rename = "shortLongTermDebtTotal")]
    pub short_long_term_debt_total: String,
    
    /// Other current liabilities
    #[serde(rename = "otherCurrentLiabilities")]
    pub other_current_liabilities: String,
    
    /// Other non-current liabilities
    #[serde(rename = "otherNonCurrentLiabilities")]
    pub other_non_current_liabilities: String,
    
    /// Total shareholder equity
    #[serde(rename = "totalShareholderEquity")]
    pub total_shareholder_equity: String,
    
    /// Treasury stock
    #[serde(rename = "treasuryStock")]
    pub treasury_stock: String,
    
    /// Retained earnings
    #[serde(rename = "retainedEarnings")]
    pub retained_earnings: String,
    
    /// Common stock
    #[serde(rename = "Equity")]
    pub common_stock: String,
    
    /// Common stock shares outstanding
    #[serde(rename = "EquitySharesOutstanding")]
    pub common_stock_shares_outstanding: String,
}

/// Cash flow statement data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CashFlow {
    /// Stock symbol
    pub symbol: String,
    
    /// Annual reports
    #[serde(rename = "annualReports")]
    pub annual_reports: Vec<CashFlowReport>,
    
    /// Quarterly reports
    #[serde(rename = "quarterlyReports")]
    pub quarterly_reports: Vec<CashFlowReport>,
}

/// Individual cash flow report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CashFlowReport {
    /// Fiscal date ending
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    
    /// Reported currency
    #[serde(rename = "reportedCurrency")]
    pub reported_currency: String,
    
    /// Operating cashflow
    #[serde(rename = "operatingCashflow")]
    pub operating_cashflow: String,
    
    /// Payments for operating activities
    #[serde(rename = "paymentsForOperatingActivities")]
    pub payments_for_operating_activities: String,
    
    /// Proceeds from operating activities
    #[serde(rename = "proceedsFromOperatingActivities")]
    pub proceeds_from_operating_activities: String,
    
    /// Change in operating liabilities
    #[serde(rename = "changeInOperatingLiabilities")]
    pub change_in_operating_liabilities: String,
    
    /// Change in operating assets
    #[serde(rename = "changeInOperatingAssets")]
    pub change_in_operating_assets: String,
    
    /// Depreciation, depletion and amortization
    #[serde(rename = "depreciationDepletionAndAmortization")]
    pub depreciation_depletion_and_amortization: String,
    
    /// Capital expenditures
    #[serde(rename = "capitalExpenditures")]
    pub capital_expenditures: String,
    
    /// Change in receivables
    #[serde(rename = "changeInReceivables")]
    pub change_in_receivables: String,
    
    /// Change in inventory
    #[serde(rename = "changeInInventory")]
    pub change_in_inventory: String,
    
    /// Profit loss
    #[serde(rename = "profitLoss")]
    pub profit_loss: String,
    
    /// Cashflow from investment
    #[serde(rename = "cashflowFromInvestment")]
    pub cashflow_from_investment: String,
    
    /// Cashflow from financing
    #[serde(rename = "cashflowFromFinancing")]
    pub cashflow_from_financing: String,
    
    /// Proceeds from repayments of short term debt
    #[serde(rename = "proceedsFromRepaymentsOfShortTermDebt")]
    pub proceeds_from_repayments_of_short_term_debt: String,
    
    /// Payments for repurchase of common stock
    #[serde(rename = "paymentsForRepurchaseOfCommonStock")]
    pub payments_for_repurchase_of_common_stock: String,
    
    /// Payments for repurchase of equity
    #[serde(rename = "paymentsForRepurchaseOfEquity")]
    pub payments_for_repurchase_of_equity: String,
    
    /// Payments for repurchase of preferred stock
    #[serde(rename = "paymentsForRepurchaseOfPreferredStock")]
    pub payments_for_repurchase_of_preferred_stock: String,
    
    /// Dividend payout
    #[serde(rename = "dividendPayout")]
    pub dividend_payout: String,
    
    /// Dividend payout common stock
    #[serde(rename = "dividendPayoutCommonStock")]
    pub dividend_payout_common_stock: String,
    
    /// Dividend payout preferred stock
    #[serde(rename = "dividendPayoutPreferredStock")]
    pub dividend_payout_preferred_stock: String,
    
    /// Proceeds from issuance of common stock
    #[serde(rename = "proceedsFromIssuanceOfCommonStock")]
    pub proceeds_from_issuance_of_common_stock: String,
    
    /// Proceeds from issuance of long term debt and capital securities net
    #[serde(rename = "proceedsFromIssuanceOfLongTermDebtAndCapitalSecuritiesNet")]
    pub proceeds_from_issuance_of_long_term_debt_and_capital_securities_net: String,
    
    /// Proceeds from issuance of preferred stock
    #[serde(rename = "proceedsFromIssuanceOfPreferredStock")]
    pub proceeds_from_issuance_of_preferred_stock: String,
    
    /// Proceeds from repurchase of equity
    #[serde(rename = "proceedsFromRepurchaseOfEquity")]
    pub proceeds_from_repurchase_of_equity: String,
    
    /// Proceeds from sale of treasury stock
    #[serde(rename = "proceedsFromSaleOfTreasuryStock")]
    pub proceeds_from_sale_of_treasury_stock: String,
    
    /// Change in cash and cash equivalents
    #[serde(rename = "changeInCashAndCashEquivalents")]
    pub change_in_cash_and_cash_equivalents: String,
    
    /// Change in exchange rate
    #[serde(rename = "changeInExchangeRate")]
    pub change_in_exchange_rate: String,
    
    /// Net income
    #[serde(rename = "netIncome")]
    pub net_income: String,
}

/// Earnings data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Earnings {
    /// Stock symbol
    pub symbol: String,
    
    /// Annual earnings
    #[serde(rename = "annualEarnings")]
    pub annual_earnings: Vec<AnnualEarnings>,
    
    /// Quarterly earnings
    #[serde(rename = "quarterlyEarnings")]
    pub quarterly_earnings: Vec<QuarterlyEarnings>,
}

/// Annual earnings report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnualEarnings {
    /// Fiscal date ending
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    
    /// Reported EPS
    #[serde(rename = "reportedEPS")]
    pub reported_eps: String,
}

/// Quarterly earnings report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuarterlyEarnings {
    /// Fiscal date ending
    #[serde(rename = "fiscalDateEnding")]
    pub fiscal_date_ending: String,
    
    /// Reported date
    #[serde(rename = "reportedDate")]
    pub reported_date: String,
    
    /// Reported EPS
    #[serde(rename = "reportedEPS")]
    pub reported_eps: String,
    
    /// Estimated EPS
    #[serde(rename = "estimatedEPS")]
    pub estimated_eps: String,
    
    /// Surprise
    #[serde(rename = "surprise")]
    pub surprise: String,
    
    /// Surprise percentage
    #[serde(rename = "surprisePercentage")]
    pub surprise_percentage: String,
}

/// Top gainers, losers, and most active stocks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopGainersLosers {
    /// Metadata
    pub metadata: String,
    
    /// Last updated timestamp
    pub last_updated: String,
    
    /// Top gainers
    pub top_gainers: Vec<StockMover>,
    
    /// Top losers
    pub top_losers: Vec<StockMover>,
    
    /// Most actively traded
    pub most_actively_traded: Vec<StockMover>,
}

/// Individual stock mover (gainer/loser/active)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StockMover {
    /// Stock ticker
    pub ticker: String,
    
    /// Current price
    pub price: String,
    
    /// Price change amount
    pub change_amount: String,
    
    /// Price change percentage
    pub change_percentage: String,
    
    /// Trading volume
    pub volume: String,
}

/// Listing status response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListingStatus {
    /// List of securities
    pub data: Vec<SecurityListing>,
}

/// Individual security listing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityListing {
    /// Symbol
    pub symbol: String,
    
    /// Company name
    pub name: String,
    
    /// Exchange
    pub exchange: String,
    
    /// Asset type
    pub asset_type: String,
    
    /// IPO date
    pub ipo_date: String,
    
    /// Delisting date (if applicable)
    pub delisting_date: Option<String>,
    
    /// Status (Active/Delisted)
    pub status: String,
}

/// Earnings calendar
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarningsCalendar {
    /// Earnings events
    pub data: Vec<EarningsEvent>,
}

/// Individual earnings event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarningsEvent {
    /// Symbol
    pub symbol: String,
    
    /// Company name
    pub name: String,
    
    /// Report date
    pub report_date: String,
    
    /// Fiscal date ending
    pub fiscal_date_ending: String,
    
    /// Estimate
    pub estimate: String,
    
    /// Currency
    pub currency: String,
}

/// IPO calendar
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpoCalendar {
    /// IPO events
    pub data: Vec<IpoEvent>,
}

/// Individual IPO event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpoEvent {
    /// Symbol
    pub symbol: String,
    
    /// Company name
    pub name: String,
    
    /// IPO date
    pub ipo_date: String,
    
    /// Price range low
    pub price_range_low: String,
    
    /// Price range high
    pub price_range_high: String,
    
    /// Currency
    pub currency: String,
    
    /// Exchange
    pub exchange: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_company_overview_deserialization() {
        let json = r#"{
            "Symbol": "AAPL",
            "AssetType": "Common Stock",
            "Name": "Apple Inc",
            "Description": "Apple Inc is an American multinational technology company",
            "CIK": "320193",
            "Exchange": "NASDAQ",
            "Currency": "USD",
            "Country": "USA",
            "Sector": "TECHNOLOGY",
            "Industry": "Consumer Electronics",
            "Address": "One Apple Park Way, Cupertino, CA, United States, 95014",
            "FiscalYearEnd": "September",
            "LatestQuarter": "2024-03-31",
            "MarketCapitalization": "3000000000000",
            "EBITDA": "123000000000",
            "PERatio": "25.5",
            "PEGRatio": "2.1",
            "BookValue": "4.9",
            "DividendPerShare": "0.96",
            "DividendYield": "0.0044",
            "EPS": "6.57",
            "RevenuePerShareTTM": "24.32",
            "ProfitMargin": "0.270",
            "OperatingMarginTTM": "0.302",
            "ReturnOnAssetsTTM": "0.199",
            "ReturnOnEquityTTM": "1.475",
            "RevenueTTM": "394328000000",
            "GrossProfitTTM": "169148000000",
            "DilutedEPSTTM": "6.57",
            "QuarterlyEarningsGrowthYOY": "0.020",
            "QuarterlyRevenueGrowthYOY": "-0.043",
            "AnalystTargetPrice": "195.5",
            "TrailingPE": "25.5",
            "ForwardPE": "24.2",
            "PriceToSalesRatioTTM": "7.8",
            "PriceToBookRatio": "35.5",
            "EVToRevenue": "7.6",
            "EVToEBITDA": "22.1",
            "Beta": "1.25",
            "52WeekHigh": "199.62",
            "52WeekLow": "164.08",
            "50DayMovingAverage": "186.5",
            "200DayMovingAverage": "180.2",
            "SharesOutstanding": "15908000000",
            "DividendDate": "2024-05-16",
            "ExDividendDate": "2024-05-10"
        }"#;
        
        let overview: CompanyOverview = serde_json::from_str(json).unwrap();
        assert_eq!(overview.symbol, "AAPL");
        assert_eq!(overview.name, "Apple Inc");
        assert_eq!(overview.sector, "TECHNOLOGY");
    }
}
