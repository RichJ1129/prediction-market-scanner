# Prediction Market Scanner

A Rust-based toolkit for analyzing Polymarket prediction markets and discovering profitable traders.

## Quick Start

```bash
# Auto-scan for profitable wallets (easiest way)
cargo run -- --scan

# Continuous scanning - keeps finding profitable wallets until you stop (Ctrl+C)
cargo run -- --scan 20000 100 --continuous

# Or analyze a specific wallet
cargo run -- 0x<wallet_address>
```

## Features

### 1. Arbitrage Scanner
Continuously scans Polymarket for arbitrage opportunities where YES + NO prices total less than $1.00.

### 2. Profitable Wallet Scanner
Automatically discovers and analyzes profitable traders on Polymarket:
- Filters for genuinely profitable wallets (10+ resolved positions, ROI > 10%, profit > $50)
- Win rate analysis with statistical significance
- Return on Investment (ROI) tracking
- Position profitability metrics
- Suspicious pattern detection (high win rates, unusual profit patterns)
- Continuous scanning mode to accumulate results over time
- Displays usernames when available

## Installation

```bash
cargo build --release
```

## Overview

This scanner helps you discover consistently profitable traders on Polymarket by:
- Analyzing thousands of active wallets automatically
- Filtering for statistically significant profitability
- Identifying suspicious trading patterns
- Running continuously to accumulate results over time

## Usage

### 1. Auto-Scan for Profitable Wallets (Recommended)

Automatically finds and analyzes active wallets:

```bash
cargo run -- --scan [sample_size] [max_wallets] [--continuous]
```

**Parameters:**
- `sample_size` (default: 5000) - Number of recent trades to fetch for wallet discovery
- `max_wallets` (default: 30) - Maximum number of wallets to analyze per scan
- `--continuous` (optional) - Run continuously, accumulating profitable wallets over time

**Examples:**
```bash
# Default: scan 5000 trades, analyze top 30 wallets
cargo run -- --scan

# Scan 20,000 trades, analyze top 100 wallets
cargo run -- --scan 20000 100

# Continuous mode: keeps scanning until you stop (Ctrl+C)
cargo run -- --scan 20000 100 --continuous

# Large one-time scan
cargo run -- --scan 50000 200
```

**How it works:**
1. Fetches recent trades from Polymarket
2. Identifies the most active wallets (sorted by trade count)
3. Loads 15,000 most recent resolved markets (optimized for speed)
4. Analyzes each wallet's trading history
5. **Reports only genuinely profitable wallets** meeting all criteria:
   - 10+ resolved positions (statistical significance)
   - ROI > 10% (meaningful profitability)
   - Net profit > $50 (filters out lucky small bets)
6. Shows usernames when available
7. In continuous mode: repeats immediately, avoiding duplicate analysis

**Output includes:**
- Wallet address (and username if available)
- Win rate percentage
- ROI (Return on Investment)
- Total invested and net profit
- Number of resolved positions
- Red flags for suspicious patterns (extremely high win rates, etc.)

**Profitability Thresholds:**
- Minimum 10 resolved positions (ensures statistical significance)
- Minimum 10% ROI (filters out break-even traders)
- Minimum $50 net profit (excludes lucky small wins)

**Note**: Genuinely profitable wallets are rare (~2-5% of active traders). Use continuous mode or larger sample sizes to find more results.

### 2. Analyze a Specific Wallet

```bash
cargo run -- <wallet_address>
```

Example:
```bash
cargo run -- 0x3a57792d11b5b578384f260d73c12eec795afb43
```

The analyzer will:
1. Fetch all trades for the wallet
2. Load resolved markets to determine outcomes
3. Calculate win rates and profitability metrics
4. Flag suspicious patterns

### 3. Run Arbitrage Scanner

```bash
cargo run
```

This will continuously scan for arbitrage opportunities every 10 seconds.

## How Insider Detection Works

The wallet analyzer identifies potential insiders by detecting these red flags:

### 1. Abnormally High Win Rate
- **Normal**: 50-60% win rate
- **Suspicious**: >65% win rate
- **Highly Suspicious**: >75% win rate

### 2. High ROI with Significant Capital
- Large returns (>50% ROI) with substantial capital deployed (>$1,000)
- Indicates consistent profitability beyond luck

### 3. Consistent Performance Across Many Markets
- Multiple wins with high win rate (>70% with 15+ wins)
- Statistical improbability suggests information advantage

### 4. Asymmetric Profit Patterns
- Average wins significantly larger than average losses (>2x)
- May indicate selective betting on high-confidence insider information

## Example Output

### Single Scan Mode
```
Polymarket Insider Scanner
==========================

✓ Loaded 15000 resolved markets in 35.2s

[100/100] Analyzing wallets...

================================================================================
SCAN SUMMARY
================================================================================

Scanned wallets: 100
Profitable wallets found: 8

================================================================================
PROFITABLE WALLETS (SORTED BY ROI)
================================================================================

1. 0x1234567890abcdef... (@ProTrader)
   Win Rate: 82.2% | ROI: 86.4% | Resolved Positions: 45
   Total Invested: $15,420.50 | Net Profit: $13,329.50
   ⚠️  Red Flags:
     • Extremely high win rate: 82.2% (normal is ~50-60%)
     • Very high ROI: 86.4% with $15420.50 invested
     • Consistent high performance: 37 wins out of 45 resolved positions
     • Asymmetric profit pattern: avg win $425.50 vs avg loss $-125.25

2. 0xabcdef1234567890...
   Win Rate: 68.5% | ROI: 42.1% | Resolved Positions: 28
   Total Invested: $8,200.00 | Net Profit: $3,452.00

3. 0x9876543210fedcba... (@MarketKing)
   Win Rate: 71.4% | ROI: 38.7% | Resolved Positions: 21
   Total Invested: $5,500.00 | Net Profit: $2,128.50

...
```

### Continuous Scan Mode
```
Running in CONTINUOUS mode - Press Ctrl+C to stop
Will keep scanning for profitable wallets and accumulate results...

✓ Loaded 15000 resolved markets in 35.2s

🔄 Scan iteration #1
================================================================================
✓ Found 32 new wallets to analyze (skipped 0 already scanned)
[32/32] Analyzing wallets...

✨ Found 3 new profitable wallet(s) in this iteration!

================================================================================
PROFITABLE WALLETS (SORTED BY ROI)
================================================================================
[Top 20 wallets shown...]

📊 Total stats:
   Scans completed: 1
   Wallets analyzed: 32
   Profitable wallets found: 3

🔄 Starting next scan... (Press Ctrl+C to stop)

🔄 Scan iteration #2
================================================================================
...

[Press Ctrl+C]

🛑 Stopping scan...

================================================================================
FINAL RESULTS
================================================================================

Total scans: 5
Total wallets analyzed: 156
Total profitable wallets found: 12

[Shows all 12 profitable wallets sorted by ROI]
```

## Finding Wallets to Analyze

You can find wallet addresses from:

1. **Recent Trades API**:
   ```bash
   curl "https://data-api.polymarket.com/trades?limit=100" | jq -r '.[].proxyWallet'
   ```

2. **Polymarket Leaderboard**: Check top traders on Polymarket's website

3. **On-Chain Analysis**: Query Polygon blockchain for high-volume traders

4. **Market-Specific Analysis**: Focus on wallets that traded on markets where insider information was likely (e.g., corporate announcements, political decisions)

## Data Sources

- **Polymarket Gamma API**: Market data and resolutions
- **Polymarket Data API**: Trade history by wallet
- **On-Chain Data**: All trades are verified on Polygon blockchain

## Important Notes

### Limitations
- **Minimum Sample Size**: Requires at least 10 resolved positions for reliable analysis
- **Active Markets**: Only counts resolved markets (open positions not included)
- **False Positives**: Skilled traders or lucky streaks may trigger flags

### Legal & Ethical Considerations
- All data analyzed is public (on-chain)
- Tool is for research and detection purposes only
- Suspected insider trading should be reported to appropriate authorities
- Polymarket has its own compliance and monitoring systems

## Technical Details

### Architecture
- **Client** (`client.rs`): API communication with Polymarket
- **Models** (`models.rs`): Data structures for markets, trades, and performance
- **Wallet Analyzer** (`wallet_analyzer.rs`): Performance calculation and pattern detection
- **Scanner** (`scanner.rs`): Arbitrage opportunity detection

### Performance
- Concurrent market fetching using `tokio` and `futures` (10 concurrent requests)
- Parallel market scanning using `rayon`
- Optimized to fetch only 15,000 most recent resolved markets (vs 233,000+ total)
- Progress indicators for long-running operations
- 30-second timeout per request to prevent hanging
- Typical market loading time: 30-60 seconds (one-time per session)
- Typical wallet analysis time: 5-10 seconds per wallet
- Continuous mode: no delay between iterations, maximum throughput

## Tips for Finding Profitable Wallets

1. **Use continuous mode** - Profitable wallets are rare, so let it run for hours to accumulate results
2. **Increase sample size** - More trades = more wallet discovery opportunities
3. **Increase max_wallets** - Analyze more wallets per scan (e.g., 100-200)
4. **Run during high activity** - More trading volume during major events
5. **Let resolved markets accumulate** - Wallets need time for positions to resolve

Example for maximum discovery:
```bash
cargo run -- --scan 50000 200 --continuous
```

## Contributing

Improvements welcome! Areas for enhancement:
- Machine learning models for insider detection
- Historical trend analysis
- Market-specific risk patterns
- Database caching for resolved markets
- API rate limiting improvements
- More sophisticated profitability metrics

## License

MIT

## Repository

GitHub: [prediction-market-scanner](https://github.com/RichJ1129/prediction-market-scanner)

## Disclaimer

This tool is for educational and research purposes. It does not constitute financial or legal advice. Always conduct your own research and consult professionals before making investment decisions or reporting suspected fraud.
