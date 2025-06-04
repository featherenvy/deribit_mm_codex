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

The backlog in `docs/delivery/backlog.md` breaks the project into one Product
Backlog Item (PBI&nbsp;1) with seven tasks. These tasks guide the initial
implementation phase:

1. **Setup Deribit API connectivity** – implement WebSocket and HTTP wrappers
   ([1-1](docs/delivery/1/1-1.md)).
2. **Implement quoting algorithm** – compute bid/ask prices using inventory and
   volatility signals ([1-2](docs/delivery/1/1-2.md)).
3. **Implement order management** – event-driven placement and cancellation of
   orders ([1-3](docs/delivery/1/1-3.md)).
4. **Risk management and hedging** – enforce inventory limits and optional cross
   instrument hedge ([1-4](docs/delivery/1/1-4.md)).
5. **Execution and latency tactics** – latency-aware quoting and adverse
   selection protections ([1-5](docs/delivery/1/1-5.md)).
6. **Configuration and logging** – configuration file and structured logging
   framework ([1-6](docs/delivery/1/1-6.md)).
7. **E2E CoS Test** – integration test verifying acceptance criteria
   ([1-7](docs/delivery/1/1-7.md)).

The research document contains pseudocode for order management loops and additional details that should guide the implementation【F:RESEARCH.md†L136-L164】.

Feel free to iterate on the design, but keep to the principles of using Deribit-only data and maintaining delta neutrality while capturing the spread.
