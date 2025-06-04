# Deribit Delta-Neutral Market Maker

This repository is for building a Rust implementation of the delta-neutral market making strategy described in `RESEARCH.md`. The strategy exclusively uses Deribit data and aims to maintain near zero BTC exposure while collecting the bid/ask spread.

See [PROJECT_POLICY.md](PROJECT_POLICY.md) for contribution workflow and task structure rules.
## Strategy Summary

- The research proposes **passive quoting** enhanced with occasional hybrid actions. All signals come from Deribit order books and trades, without external feeds. The goal is to "combine passive quoting (earning the bid-ask spread by resting limit orders) with hybrid tactics" while watching microstructure signals such as depth, top-of-book moves and recent trades【F:RESEARCH.md†L5-L16】.
- Quoting centers around a **reservation price**. Starting from the mid-price `(best_bid + best_ask) / 2`, we shift the reference according to inventory using a factor `γ` and recent volatility, ensuring long inventory shifts quotes downward and short inventory shifts upward【F:RESEARCH.md†L30-L38】.
- The spread width is set dynamically. The research suggests using recent volatility, fee considerations and order book imbalance to widen or tighten quotes while enforcing minimum/maximum bounds【F:RESEARCH.md†L40-L48】.
- Order placement is **event-driven**. Quote updates trigger on order book changes, fills and periodic timers. Post-only limit orders are preferred; orders are cancelled or modified when stale or when other makers undercut us, and the opposite side is cancelled after a fill to avoid double fills【F:RESEARCH.md†L100-L130】.
- **Inventory limits** control risk. With 0.9 BTC capital, the paper recommends keeping exposure within ±0.3 BTC and halting quotes on a side once that limit is reached. Hedging between the perpetual and expiring futures can neutralize directional risk when needed【F:RESEARCH.md†L170-L187】.
- Profitability relies on earning spread with minimal taker trades. The expected daily return for moderate risk is estimated around 0.002–0.005 BTC after fees, though actual performance will vary with market conditions【F:RESEARCH.md†L244-L267】.

## Project Goals for the AI

1. **Rust codebase** – implement the strategy using async Rust (e.g. Tokio).
2. **Deribit connectivity** – subscribe to WebSocket feeds for order books and trades and send orders through the HTTP or RPC API.
3. **Modules to implement**:
   - `exchange`: WebSocket client and REST/RPC wrapper
   - `strategy`: quoting logic, inventory skew, volatility-based spread
   - `orders`: state tracking and event-driven management
   - `risk`: inventory limits, hedging logic and kill-switch thresholds
4. **Configuration** – expose parameters such as `base_spread`, `γ`, order size and risk thresholds via a config file.
5. **Testing and simulation** – include basic unit tests and allow running on Deribit testnet first.

The research document contains pseudocode for order management loops and additional details that should guide the implementation【F:RESEARCH.md†L136-L164】.

Feel free to iterate on the design, but keep to the principles of using Deribit-only data and maintaining delta neutrality while capturing the spread.
