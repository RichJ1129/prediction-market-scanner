# Polymarket Analysis Tools

A Rust-based toolkit for analyzing Polymarket prediction markets and detecting potential insider trading patterns.

## Features

### 1. Arbitrage Scanner
Continuously scans Polymarket for arbitrage opportunities where YES + NO prices total less than $1.00.

### 2. Wallet Analyzer (Insider Detection)
Analyzes individual wallet trading performance to identify patterns consistent with insider knowledge:
- Win rate analysis
- Return on Investment (ROI) tracking
- Position profitability metrics
- Suspicious pattern detection

## Installation

```bash
cargo build --release
```

## Usage

### Analyze a Wallet for Insider Activity

```bash
cargo run <wallet_address>
```

Example:
```bash
cargo run 0x3a57792d11b5b578384f260d73c12eec795afb43
```

The analyzer will:
1. Fetch all trades for the wallet
2. Load resolved markets to determine outcomes
3. Calculate win rates and profitability metrics
4. Flag suspicious patterns

### Run Arbitrage Scanner

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

```
================================================================================
WALLET PERFORMANCE REPORT
================================================================================

Wallet: 0x1234...5678

--- Trading Activity ---
Total Trades:         156
Unique Markets:       89
Resolved Positions:   45

--- Win/Loss Record ---
Wins:                 37
Losses:               8
Win Rate:             82.2%

--- Financial Performance ---
Total Invested:       $15,420.50
Total Payout:         $28,750.00
Net Profit:           $13,329.50
ROI:                  86.4%
Avg Profit per Win:   $425.50
Avg Loss per Loss:    -$125.25

================================================================================
⚠️  SUSPICIOUS ACTIVITY DETECTED
================================================================================
• Extremely high win rate: 82.2% (normal is ~50-60%)
• Very high ROI: 86.4% with $15420.50 invested
• Consistent high performance: 37 wins out of 45 resolved positions
• Asymmetric profit pattern: avg win $425.50 vs avg loss $-125.25

This wallet shows patterns consistent with potential insider knowledge.
================================================================================
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
- Concurrent market fetching using `tokio` and `futures`
- Parallel market scanning using `rayon`
- Typical analysis time: 20-30 seconds per wallet

## Contributing

Improvements welcome! Areas for enhancement:
- Machine learning models for insider detection
- Historical trend analysis
- Market-specific risk patterns
- Batch wallet analysis
- API rate limiting and caching

## License

MIT

## Disclaimer

This tool is for educational and research purposes. It does not constitute financial or legal advice. Always conduct your own research and consult professionals before making investment decisions or reporting suspected fraud.
