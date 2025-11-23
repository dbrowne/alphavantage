# Data Visualization - POC

A static, self-contained D3.js visualization showing relationships between news articles and stock symbols.

**crypto_heatmap.html** is a simple heatmap visualization of the crypto names by market cap. Highlights a problem with symbol mapping. see 

Will provide more complete instructions in a future commit.

This is my first foray into frontend work and will move this to a separate repo but for now it is actually useful for debugging and ideas

## Quick Start

### 1. Extract Data from Database

```bash
cd viz
./extract_data.sh
```

This will:
- Extract ticker symbols from article titles using regex
- Query the PostgreSQL database for the top 50 most-mentioned symbols
- Fetch 200 recent articles containing ticker symbols
- Generate relationships by matching symbols in article titles
- Save everything to `news_data.json`

### 2. View Visualization

Open `index.html` in a web browser:

```bash
# Option 1: Direct open
open index.html  # macOS
xdg-open index.html  # Linux
start index.html  # Windows

# Option 2: Simple HTTP server (recommended for production)
python3 -m http.server 8000
# Then visit: http://localhost:8000
```

## Features

### Visualization Elements

- **Nodes**:
  - 🔵 **Blue circles** = Stock symbols (NVDA, TSLA, AAPL, etc.)
    - Size proportional to number of article mentions
    - Larger nodes = more frequently mentioned symbols
  - 🟣 **Purple circles** = News articles
    - Smaller, uniform size
    - Click to open article URL

- **Links** (Edges):
  - ⚪ Gray lines connecting symbols to articles that mention them
  - Derived from regex pattern matching in article titles

### Interactive Features

1. **Hover**: View detailed information:
   - Symbols: Show ticker, company name, and article count
   - Articles: Show full title and publish date
2. **Click**: Click article nodes to open the source URL in a new tab
3. **Drag**: Drag nodes to manually arrange the graph
4. **Zoom/Pan**: Scroll to zoom, drag background to pan

### Stats Panel

Real-time statistics showing:
- Number of symbols displayed (50)
- Number of articles displayed (200)
- Number of connections (68+)

## Customization

### Adjust Time Range

Edit `extract_news_data.sql` line 30:
```sql
WHERE a.ct >= CURRENT_DATE - INTERVAL '30 days'  -- Change to '60 days', '7 days', etc.
```

### Change Symbol Limit

Edit `extract_news_data.sql` line 17:
```sql
LIMIT 25  -- Change to 50, 100, etc.
```

### Modify Visual Appearance

Edit `index.html`:

**Colors** (around line 210):
```javascript
const typeColors = {
    'Equity': '#3b82f6',        // Change colors here
    'Cryptocurrency': '#f59e0b',
    'ETF': '#10b981',
    // Add more types as needed
};
```

**Node Sizes** (around line 288):
```javascript
size: Math.sqrt(symbol.article_count) * 8 + 10  // Adjust multiplier
```

**Force Simulation** (around line 386):
```javascript
.force('charge', d3.forceManyBody().strength(-300))  // Adjust repulsion
.force('link', d3.forceLink(links).distance(100))    // Adjust link distance
```





## Performance Notes

- **Recommended limits**:
  - Symbols: 25-50 (current: 25)
  - Articles: 100-500 per symbol
  - Total nodes: < 1000 for smooth interaction

- **For larger datasets**:
  - Increase time to stabilize: `simulation.alphaTarget(0.01)`



## Dependencies

- **D3.js v7**: Loaded from CDN (https://d3js.org/d3.v7.min.js)
- **jq** (optional): For JSON formatting in extraction script

No build process required - it's pure HTML/JavaScript