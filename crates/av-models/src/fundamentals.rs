/*
 *
 *
 *
 *
 * MIT License
 * Copyright (c) 2025. Dwight J. Browne
 * dwight[-at-]dwightjbrowne[-dot-]com
 *
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

//! Fundamental analysis data models for company financials

use serde::{Deserialize, Serialize};

/// Company overview with key financial metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanyOverview {
  #[serde(rename = "Symbol")]
  pub symbol: String,

  #[serde(rename = "AssetType")]
  pub asset_type: String,

  #[serde(rename = "Name")]
  pub name: String,

  #[serde(rename = "Description")]
  pub description: String,

  /// Central Index Key (CIK)
  #[serde(rename = "CIK")]
  pub cik: String,

  /// Exchange where the stock is traded
  #[serde(rename = "Exchange")]
  pub exchange: String,

  #[serde(rename = "Currency")]
  pub currency: String,

  #[serde(rename = "Country")]
  pub country: String,

  #[serde(rename = "Sector")]
  pub sector: String,

  /// Industry classification
  #[serde(rename = "Industry")]
  pub industry: String,

  #[serde(rename = "Address")]
  pub address: String,

  #[serde(rename = "FiscalYearEnd")]
  pub fiscal_year_end: String,

  /// Latest quarter end date
  #[serde(rename = "LatestQuarter")]
  pub latest_quarter: String,

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

  #[serde(rename = "BookValue")]
  pub book_value: String,

  #[serde(rename = "DividendPerShare")]
  pub dividend_per_share: String,

  #[serde(rename = "DividendYield")]
  pub dividend_yield: String,

  #[serde(rename = "EPS")]
  pub eps: String,

  #[serde(rename = "RevenuePerShareTTM")]
  pub revenue_per_share_ttm: String,

  #[serde(rename = "ProfitMargin")]
  pub profit_margin: String,

  #[serde(rename = "OperatingMarginTTM")]
  pub operating_margin_ttm: String,

  #[serde(rename = "ReturnOnAssetsTTM")]
  pub return_on_assets_ttm: String,

  #[serde(rename = "ReturnOnEquityTTM")]
  pub return_on_equity_ttm: String,

  #[serde(rename = "RevenueTTM")]
  pub revenue_ttm: String,

  #[serde(rename = "GrossProfitTTM")]
  pub gross_profit_ttm: String,

  #[serde(rename = "DilutedEPSTTM")]
  pub diluted_eps_ttm: String,

  #[serde(rename = "QuarterlyEarningsGrowthYOY")]
  pub quarterly_earnings_growth_yoy: String,

  #[serde(rename = "QuarterlyRevenueGrowthYOY")]
  pub quarterly_revenue_growth_yoy: String,

  #[serde(rename = "AnalystTargetPrice")]
  pub analyst_target_price: String,

  #[serde(rename = "TrailingPE")]
  pub trailing_pe: String,

  #[serde(rename = "ForwardPE")]
  pub forward_pe: String,

  #[serde(rename = "PriceToSalesRatioTTM")]
  pub price_to_sales_ratio_ttm: String,

  #[serde(rename = "PriceToBookRatio")]
  pub price_to_book_ratio: String,

  #[serde(rename = "EVToRevenue")]
  pub ev_to_revenue: String,

  #[serde(rename = "EVToEBITDA")]
  pub ev_to_ebitda: String,

  #[serde(rename = "Beta")]
  pub beta: String,

  #[serde(rename = "52WeekHigh")]
  pub week_52_high: String,

  #[serde(rename = "52WeekLow")]
  pub week_52_low: String,

  #[serde(rename = "50DayMovingAverage")]
  pub day_50_moving_average: String,

  #[serde(rename = "200DayMovingAverage")]
  pub day_200_moving_average: String,

  #[serde(rename = "SharesOutstanding")]
  pub shares_outstanding: String,

  #[serde(rename = "DividendDate")]
  pub dividend_date: String,

  #[serde(rename = "ExDividendDate")]
  pub ex_dividend_date: String,
}

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

  #[serde(rename = "reportedCurrency")]
  pub reported_currency: String,

  #[serde(rename = "grossProfit")]
  pub gross_profit: String,

  #[serde(rename = "totalRevenue")]
  pub total_revenue: String,

  #[serde(rename = "costOfRevenue")]
  pub cost_of_revenue: String,

  #[serde(rename = "costofGoodsAndServicesSold")]
  pub cost_of_goods_and_services_sold: String,

  #[serde(rename = "operatingIncome")]
  pub operating_income: String,

  #[serde(rename = "sellingGeneralAndAdministrative")]
  pub selling_general_and_administrative: String,

  #[serde(rename = "researchAndDevelopment")]
  pub research_and_development: String,

  #[serde(rename = "operatingExpenses")]
  pub operating_expenses: String,

  #[serde(rename = "investmentIncomeNet")]
  pub investment_income_net: String,

  #[serde(rename = "netInterestIncome")]
  pub net_interest_income: String,

  #[serde(rename = "interestIncome")]
  pub interest_income: String,

  #[serde(rename = "interestExpense")]
  pub interest_expense: String,

  #[serde(rename = "nonInterestIncome")]
  pub non_interest_income: String,

  #[serde(rename = "otherNonOperatingIncome")]
  pub other_non_operating_income: String,

  #[serde(rename = "depreciation")]
  pub depreciation: String,

  #[serde(rename = "depreciationAndAmortization")]
  pub depreciation_and_amortization: String,

  #[serde(rename = "incomeBeforeTax")]
  pub income_before_tax: String,

  #[serde(rename = "incomeTaxExpense")]
  pub income_tax_expense: String,

  #[serde(rename = "interestAndDebtExpense")]
  pub interest_and_debt_expense: String,

  #[serde(rename = "netIncomeFromContinuingOperations")]
  pub net_income_from_continuing_operations: String,

  #[serde(rename = "comprehensiveIncomeNetOfTax")]
  pub comprehensive_income_net_of_tax: String,

  #[serde(rename = "ebit")]
  pub ebit: String,

  #[serde(rename = "ebitda")]
  pub ebitda: String,

  #[serde(rename = "netIncome")]
  pub net_income: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceSheet {
  pub symbol: String,

  #[serde(rename = "annualReports")]
  pub annual_reports: Vec<BalanceSheetReport>,

  #[serde(rename = "quarterlyReports")]
  pub quarterly_reports: Vec<BalanceSheetReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceSheetReport {
  #[serde(rename = "fiscalDateEnding")]
  pub fiscal_date_ending: String,

  #[serde(rename = "reportedCurrency")]
  pub reported_currency: String,

  #[serde(rename = "totalAssets")]
  pub total_assets: String,

  #[serde(rename = "totalCurrentAssets")]
  pub total_current_assets: String,

  #[serde(rename = "cashAndCashEquivalentsAtCarryingValue")]
  pub cash_and_cash_equivalents_at_carrying_value: String,

  #[serde(rename = "cashAndShortTermInvestments")]
  pub cash_and_short_term_investments: String,

  #[serde(rename = "inventory")]
  pub inventory: String,

  #[serde(rename = "currentNetReceivables")]
  pub current_net_receivables: String,

  #[serde(rename = "totalNonCurrentAssets")]
  pub total_non_current_assets: String,

  #[serde(rename = "propertyPlantEquipment")]
  pub property_plant_equipment: String,

  #[serde(rename = "accumulatedDepreciationAmortizationPPE")]
  pub accumulated_depreciation_amortization_ppe: String,

  #[serde(rename = "intangibleAssets")]
  pub intangible_assets: String,

  #[serde(rename = "intangibleAssetsExcludingGoodwill")]
  pub intangible_assets_excluding_goodwill: String,

  #[serde(rename = "goodwill")]
  pub goodwill: String,

  #[serde(rename = "investments")]
  pub investments: String,

  #[serde(rename = "longTermInvestments")]
  pub long_term_investments: String,

  #[serde(rename = "shortTermInvestments")]
  pub short_term_investments: String,

  #[serde(rename = "otherCurrentAssets")]
  pub other_current_assets: String,

  #[serde(rename = "otherNonCurrentAssets")]
  pub other_non_current_assets: String,

  #[serde(rename = "totalLiabilities")]
  pub total_liabilities: String,

  #[serde(rename = "totalCurrentLiabilities")]
  pub total_current_liabilities: String,

  #[serde(rename = "currentAccountsPayable")]
  pub current_accounts_payable: String,

  #[serde(rename = "deferredRevenue")]
  pub deferred_revenue: String,

  #[serde(rename = "currentDebt")]
  pub current_debt: String,

  #[serde(rename = "shortTermDebt")]
  pub short_term_debt: String,

  #[serde(rename = "totalNonCurrentLiabilities")]
  pub total_non_current_liabilities: String,

  #[serde(rename = "capitalLeaseObligations")]
  pub capital_lease_obligations: String,

  #[serde(rename = "longTermDebt")]
  pub long_term_debt: String,

  #[serde(rename = "currentLongTermDebt")]
  pub current_long_term_debt: String,

  #[serde(rename = "longTermDebtNoncurrent")]
  pub long_term_debt_noncurrent: String,

  #[serde(rename = "shortLongTermDebtTotal")]
  pub short_long_term_debt_total: String,

  #[serde(rename = "otherCurrentLiabilities")]
  pub other_current_liabilities: String,

  #[serde(rename = "otherNonCurrentLiabilities")]
  pub other_non_current_liabilities: String,

  #[serde(rename = "totalShareholderEquity")]
  pub total_shareholder_equity: String,

  #[serde(rename = "treasuryStock")]
  pub treasury_stock: String,

  #[serde(rename = "retainedEarnings")]
  pub retained_earnings: String,

  #[serde(rename = "Equity")]
  pub common_stock: String,

  #[serde(rename = "EquitySharesOutstanding")]
  pub common_stock_shares_outstanding: String,
}

/// Cash flow statement data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CashFlow {
  pub symbol: String,

  #[serde(rename = "annualReports")]
  pub annual_reports: Vec<CashFlowReport>,

  #[serde(rename = "quarterlyReports")]
  pub quarterly_reports: Vec<CashFlowReport>,
}

/// Individual cash flow report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CashFlowReport {
  #[serde(rename = "fiscalDateEnding")]
  pub fiscal_date_ending: String,

  #[serde(rename = "reportedCurrency")]
  pub reported_currency: String,

  #[serde(rename = "operatingCashflow")]
  pub operating_cashflow: String,

  #[serde(rename = "paymentsForOperatingActivities")]
  pub payments_for_operating_activities: String,

  #[serde(rename = "proceedsFromOperatingActivities")]
  pub proceeds_from_operating_activities: String,

  #[serde(rename = "changeInOperatingLiabilities")]
  pub change_in_operating_liabilities: String,

  #[serde(rename = "changeInOperatingAssets")]
  pub change_in_operating_assets: String,

  #[serde(rename = "depreciationDepletionAndAmortization")]
  pub depreciation_depletion_and_amortization: String,

  #[serde(rename = "capitalExpenditures")]
  pub capital_expenditures: String,

  #[serde(rename = "changeInReceivables")]
  pub change_in_receivables: String,

  #[serde(rename = "changeInInventory")]
  pub change_in_inventory: String,

  #[serde(rename = "profitLoss")]
  pub profit_loss: String,

  #[serde(rename = "cashflowFromInvestment")]
  pub cashflow_from_investment: String,

  #[serde(rename = "cashflowFromFinancing")]
  pub cashflow_from_financing: String,

  #[serde(rename = "proceedsFromRepaymentsOfShortTermDebt")]
  pub proceeds_from_repayments_of_short_term_debt: String,

  #[serde(rename = "paymentsForRepurchaseOfCommonStock")]
  pub payments_for_repurchase_of_common_stock: String,

  #[serde(rename = "paymentsForRepurchaseOfEquity")]
  pub payments_for_repurchase_of_equity: String,

  #[serde(rename = "paymentsForRepurchaseOfPreferredStock")]
  pub payments_for_repurchase_of_preferred_stock: String,

  #[serde(rename = "dividendPayout")]
  pub dividend_payout: String,

  #[serde(rename = "dividendPayoutCommonStock")]
  pub dividend_payout_common_stock: String,

  #[serde(rename = "dividendPayoutPreferredStock")]
  pub dividend_payout_preferred_stock: String,

  #[serde(rename = "proceedsFromIssuanceOfCommonStock")]
  pub proceeds_from_issuance_of_common_stock: String,

  #[serde(rename = "proceedsFromIssuanceOfLongTermDebtAndCapitalSecuritiesNet")]
  pub proceeds_from_issuance_of_long_term_debt_and_capital_securities_net: String,

  #[serde(rename = "proceedsFromIssuanceOfPreferredStock")]
  pub proceeds_from_issuance_of_preferred_stock: String,

  #[serde(rename = "proceedsFromRepurchaseOfEquity")]
  pub proceeds_from_repurchase_of_equity: String,

  #[serde(rename = "proceedsFromSaleOfTreasuryStock")]
  pub proceeds_from_sale_of_treasury_stock: String,

  #[serde(rename = "changeInCashAndCashEquivalents")]
  pub change_in_cash_and_cash_equivalents: String,

  #[serde(rename = "changeInExchangeRate")]
  pub change_in_exchange_rate: String,

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
