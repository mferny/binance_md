# Binance Market Data Connector

This is a simple market data connector that connects to the Binance Spot API and publishes:

- Best deals (best bid and ask prices).
- Aggregated trades.
- The first 5 levels of a locally built order book.

The connector adheres to the official Binance Spot API WebSocket Streams specification:
[Binance Spot API Docs](https://developers.binance.com/docs/binance-spot-api-docs/web-socket-streams).

Recovery mechanism includes a 5s timeout on market data absence in publishing. 
When this timeout is reached, connector will recover the book with snapshot. 
Also, snapshot recovery is used on program start.

## Usage

To run the connector, execute the following command in the project directory:

```bash
cargo run
```

## Example output
```bash
AggTrade - Symbol: BTCUSDT, Price: 96299.62000000, Quantity: 0.00469000, Is Market Maker: true

AggTrade - Symbol: BTCUSDT, Price: 96299.63000000, Quantity: 0.00006000, Is Market Maker: false

AggTrade - Symbol: BTCUSDT, Price: 96299.62000000, Quantity: 0.00014000, Is Market Maker: true

Best Deal:
  Last Update ID: 57092430321
  Best Bid: Price: 96299.62000000, Quantity: 1.65153000
  Best Ask: Price: 96299.63000000, Quantity: 1.69991000

Order Book:
Bids:
  Price: 96299.62, Qty: 1.65153
  Price: 96299.5, Qty: 0.0001
  Price: 96299.49, Qty: 0.0001
  Price: 96299.36, Qty: 0.0001
  Price: 96299.35, Qty: 0.0001
Asks:
  Price: 96299.63, Qty: 1.69991
  Price: 96299.97, Qty: 0.00016
  Price: 96299.98, Qty: 0.00253
  Price: 96299.99, Qty: 0.05055
  Price: 96300, Qty: 0.03028
```
