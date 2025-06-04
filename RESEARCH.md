# Market Making Strategy for BTC Futures on Deribit

## Introduction and Strategy Overview

This report outlines a market making strategy for BTC futures on Deribit (including the perpetual swap and expiring futures) tailored for a developer with ~0.9 BTC of dedicated capital. The strategy is exchange-exclusive to Deribit – all trading, hedging, and data signals come from Deribit’s own order book and trade feed, with no external indicators. It combines passive quoting (earning the bid-ask spread by resting limit orders) with hybrid tactics (occasionally adjusting or aggressing orders) to optimize performance. The approach leverages market microstructure signals – such as order book depth, top-of-book changes, recent trade flow, spread patterns, and funding rate skews – to make real-time decisions.

Given the non-colocated setup (a VPS with higher latency than on-site servers), the strategy emphasizes robust risk management and adverse-selection avoidance over pure speed. We will detail how to quote prices, manage orders, control inventory risk, and handle execution nuances. Pseudocode and formulas are included to illustrate the implementation logic in Python. The goal is to reliably capture small profits from the spread while controlling risks, with realistic expectations on profitability after fees and under various market conditions.

## Deribit Microstructure and Environment

Deribit Exchange Context: Deribit is a leading crypto derivatives exchange offering BTC perpetual swaps and fixed-expiry futures (e.g. quarterly). Markets operate 24/7 with deep but competitive order books. Deribit’s fee structure historically uses a maker-taker model to encourage liquidity: for instance, the BTC perpetual swap charged takers about 0.075% and offered makers a 0.025% rebate per trade. (Standard accounts today pay roughly 0.025% each side, but the principle remains that passive orders are far cheaper than aggressive ones.) This means a market maker gains a small edge on fees by providing liquidity, but must manage the information disadvantage that comes with being a maker. By definition, a liquidity taker often has more information or urgency than the maker resting orders, so makers risk being “picked off” just before adverse price moves. The strategy accounts for this by using cautious quoting and protective measures.

Order Book Structure: BTC futures on Deribit are quoted in USD terms. Price ticks are small (e.g. $0.10 or $0.50 increments), and spreads on the perpetual are typically very tight (often a few dollars) in normal conditions. Expiring futures may have slightly different spreads or depth, especially if farther from expiry. The top-of-book (best bid and ask) updates frequently, and HFT market makers are active, especially during volatile periods. As a non-colocated participant (perhaps ~10-100ms latency versus sub-millisecond for colocation), our strategy will avoid latency races and instead rely on intelligent positioning of orders and reactive adjustments.

Microstructure Signals Used: We do not rely on any external price feeds or technical indicators; instead, our strategy continuously monitors Deribit’s market data for insights:
-	Order Book Depth & Imbalance: The relative volume on bid vs ask side can indicate pressure. For example, if bids are much larger than asks at multiple levels, it may signal short-term support (or conversely, a potential large seller hiding on asks). We use depth imbalance to skew quotes slightly (explained later).
-	Top-of-Book Dynamics: The frequency and direction of best bid/ask changes are observed. Rapid upward moves in the best bid or frequent ask lifts suggest bullish momentum, whereas falling offers or heavy hits on the bids suggest bearish momentum. The strategy can temporarily widen or pull quotes if the top-of-book is moving too fast (to avoid being caught in a sharp move).
-	Recent Trade Flow: We watch the tape of recent trades (buy vs sell market orders). If we see, for example, a flurry of aggressive buys lifting the ask, it indicates short-term upward pressure. The strategy might respond by being less aggressive on its sell quotes (widening asks or even pausing quoting on that side momentarily) to avoid selling too cheap into a rally. Conversely, heavy sell sweeps would cause caution on the bid side.
-	Spread and Fill Patterns: The prevailing bid-ask spread and how often our orders get filled is monitored. In stable periods with a wide spread, there’s room to quote inside and capture profit; in extremely tight spread conditions, we must decide if it’s worth matching the best quotes or waiting. We also track our fill rate – e.g., if our orders never fill, we may be quoting too far from the market; if we get filled too frequently (especially and then see price go against us), we might be quoting too tight and need to widen out to protect from toxic flow.
-	Funding Rate Skew (Perpetuals): The BTC perpetual swap has a funding rate typically paid every 8 hours. This rate, determined by the perp’s premium or discount to the index, influences our strategy. For instance, if the funding rate is strongly positive (longs pay shorts a high rate), it indicates the perp price is above fair value and may eventually mean-revert down or encourage traders to short the perp. Our strategy would be more willing to hold a short inventory on the perp during such times (since it earns funding) and will be careful about holding long positions through a high funding payment. We incorporate expected funding into our quote bias (e.g., just before a funding timestamp, adjust quotes to reduce positions that would incur a cost). For expiring futures, there is no periodic funding, but they can trade at a basis (premium or discount to the index). We treat a significant basis similarly – e.g., if a future is far above the index (expensive), we lean toward short quotes on that contract, knowing it should converge down at expiry.

All these signals are derived from Deribit’s own data. No external price feeds or indicators are used, aligning with the requirement of using solely market-internal information.

## Quoting Algorithm and Price Levels

At the heart of the strategy is the quoting algorithm – how we decide the prices and sizes at which to place buy (bid) and sell (ask) orders. The goal is to quote both sides of the market to earn the spread, while adjusting these quotes based on market conditions and our inventory. Key components of the quoting logic include the base price reference, spread width, inventory skew, and dynamic adjustments:

### 1. Base Price Reference (Midpoint)

We start with a reference price around which to center our quotes. A natural choice is the mid-price, defined as `(best_bid + best_ask) / 2` from the order book. This mid-price reflects the current market consensus. If the spread is momentarily large or only one side has orders, we could use the last trade price or Deribit’s mark price as the reference.

However, the true fair price for our quoting can be adjusted by inventory (see below) and other factors. We will sometimes use a “reservation price” concept similar to Avellaneda-Stoikov’s model, which shifts the base price up or down based on our inventory. Essentially, if we are net long, our reservation price will be slightly lower than the market mid (we value the asset a bit less because we already hold inventory we need to offload), and if we are net short, the reservation price is higher than mid (we value it more because we need to buy). This ensures our quotes lean towards bringing our position back to neutral.

Inventory-Based Skew (Reservation Price): Let \(q\) be our current inventory in BTC (positive if long, negative if short). We introduce a parameter \(\gamma\) that represents inventory risk aversion – higher \(\gamma\) means we penalize holding inventory more strongly. We also estimate short-term volatility \(\sigma\) (e.g., the standard deviation of price movements over the last few minutes). A simplified reservation price formula could be:
\[ \text{reservation_price} = \text{mid_price} - \gamma \cdot q \cdot \sigma^2 \]
inspired by the optimal market making models. Here, if \(q > 0\) (long inventory), the reservation_price is below mid_price (we’ll quote sells lower to get rid of longs, and place buys more cautiously). If \(q < 0\) (short inventory), reservation_price is above mid (we value it more because we need to buy). This ensures our quotes lean towards bringing our position back to neutral. In practice, we can tune the adjustment \(\gamma \sigma^2\) (or another simpler factor) so that, say, holding 0.5 BTC inventory might shift our quote center by a few dollars. The exact formula can be calibrated, but the qualitative effect is: long inventory -> shift quotes downward; short inventory -> shift quotes upward. This skewing of quotes helps us mean-revert our inventory by offering slightly better prices to the market on the side that will reduce our position. (We essentially give up a tiny edge to get back to neutral faster, which is worthwhile to control risk.)

We will denote this adjusted reference as ref_price (which = mid_price plus any inventory or funding bias). Initially, with no inventory, ref_price = mid_price.

### 2. Spread Width Determination

Next, we decide how far apart to place our bid and ask around the reference price. The quoting width (bid-ask spread we set) is crucial: too narrow and we’ll get filled often but with razor-thin profit (and risk being picked off by informed traders); too wide and we rarely trade (missing opportunities).

We use a dynamic approach to set the half-spread \(\Delta\) (distance from ref_price to each quote):
-	Volatility-Based Spread: In more volatile markets, we widen our quotes. A simple rule is to tie the spread to recent volatility. For example, we can set \( \text{spread} = k \cdot \sigma \) (for some factor \(k\) based on risk tolerance and desired fill frequency). If BTC’s short-term volatility is high, this yields a larger spread to protect against price swings (reducing adverse selection risk). Conversely, in calm conditions, we tighten the spread to capture more trades. We might use a minimum spread floor as well (e.g. never quote less than, say, $5 or 0.02%).
-	Fee/Profit Consideration: We ensure the spread is enough to cover fees and yield profit. Given Deribit’s fees, if paying ~0.05% round-trip for a maker order in+out (or earning a small rebate), our spread should ideally be significantly above 0.05% of price to net a profit. For instance, at a $30,000 BTC price, 0.05% is $15. We would target a spread perhaps around $30 (0.1%) or more in normal conditions, if sustainable. In very tight competitive markets, spreads might be tighter, but then we rely on the maker rebate or high fill rate to still make money. The strategy can modulate spread to be wider if competition thins out (e.g., during volatile spikes, other makers back off, allowing a wider profitable spread).
-	Microstructure Signals Influence: We also adjust the spread or individual quote offsets based on order book signals. For example, if we detect order book imbalance – say the bid side depth is dramatically larger than the ask side – it might indicate a likely upward move (or at least that being bid is slightly safer than being ask). In that case, we could tighten the bid quote a bit (or place it slightly closer to the market) while perhaps widening the ask. Conversely, if asks outweigh bids, we might ease off on bidding. Recent trade flow also matters: if buys are dominating (aggressive takers lifting offers), we may widen the ask (or even not quote aggressively at all on ask) because the market may be about to move up (we don’t want to sell right before a price jump). Our algorithm might implement a simple rule like: if last \(X\) trades were all buys and price upticked, then increase \(\Delta\) on the ask side (widen ask) and maybe tighten the bid to catch the pullback. These adjustments effectively skew the spread to be asymmetric when needed (e.g., spread might be $50 wide total, but perhaps $20 below ref price and $30 above it, if we are bullish biased at the moment or have short inventory).
-	Minimum and Maximum Width: We will define practical limits to the spread. There’s often a market-driven minimum spread (if we quote too wide relative to others, we simply won’t get any fills). And there’s a maximum beyond which we’d rather not quote at all (e.g., if the market is so volatile that a rational spread for our risk tolerance would be extremely wide, we might temporarily stop market making until things calm down). For example, we might decide if the 1-minute realized volatility annualized exceeds a certain threshold (indicating extremely turbulent market), we pause or only quote very small size with a huge spread, effectively standing aside until sanity returns.

### 3. Passive vs Hybrid Quoting Tactics

Passive Quoting: By default, our strategy is passive – it places limit orders and waits for the market to come to us. We use post-only limit orders to ensure we don’t accidentally cross the spread and incur taker fees. The core quoting cycle will continuously maintain one buy and one sell order at our determined bid and ask prices (and possibly at multiple levels, as discussed later). Passive orders earn the spread if executed, and potentially a maker rebate, which is the bread-and-butter of the strategy’s profits.

Hybrid (Passive/Aggressive) Adjustments: Pure passive market making can sometimes be suboptimal, especially when the market moves quickly or our inventory gets unbalanced. Thus, we incorporate a few hybrid elements – where we momentarily behave more aggressively or reactively:
-	Momentum Cancellation: If the market price starts moving sharply toward one of our orders (e.g., a sudden surge of trades hitting bids and our bid is about to be next), a pure passive strategy might get filled and then see the price continue dropping (adverse selection). Our strategy will detect rapid moves or large trades and pull our quotes if necessary. This isn’t exactly aggressive trading, but it’s a reactive tactic to avoid getting hit in a bad moment. Essentially, we won’t stubbornly sit with a quote if we sense it’s about to be “picked off” by someone with better information; we cancel and wait until the surge passes. We might quantify this as: if price moves more than \(Y\%\) within \(Z\) milliseconds or if a single trade larger than some threshold goes through against the side we’re quoting, we cancel that quote immediately. We can always re-enter later at a new level. This protects us from the most toxic flow.
-	Inventory-Based Aggression: If our inventory reaches an uncomfortable level (say we’ve bought a lot in a falling market), we might step out of passive mode to reduce risk. One hybrid approach is to cross the spread with a market or aggressive limit order to offload inventory when risk limits are hit. For example, if our position is long beyond our limit and the market shows continued weakness, we may send a market sell (taker order) for a portion of our inventory to quickly cut exposure. We incur a taker fee here, but this can prevent larger losses. This is a conscious trade-off: we sacrifice some profit (pay fees and possibly a worse price) to reset our risk. The strategy sets thresholds for this: e.g., if inventory > \(X\) BTC long and the last \(N\) price updates are downward, then immediately sell \(Y\) BTC at market to reduce position. This is a last resort measure; we try to manage inventory with passive skewing first, but the aggressive hedge is a backstop.
-	Join Spread Tactically: There may be times we choose to temporarily remove our quote on one side to avoid inventory increases, effectively going quasi-aggressive on the other side. For example, if we feel strongly (from microstructure signals) that price will rise, we might stop offering on the ask and only keep a bid or even chase up with the bid a bit. While we typically avoid chasing the market (since we are not trying to predict large moves, just make spread), slight aggressiveness like moving our quote in response to others can improve our fill probability when we want it. We remain mostly passive but “shade” our quotes in the direction of anticipated movement. This hybrid behavior means we are not always symmetrically making both a bid and ask – sometimes we temporarily lean to one side or adjust quotes by a few ticks in anticipation of the very immediate order flow.
-	Multiple Quote Levels: Another form of hybrid quoting is placing multiple layers of orders at different prices. For instance, we could place one bid near the best bid and another deeper (lower price). The front bid might catch small retracements, and if a larger drop happens, our deeper order might fill at a better price. This way we passively buy more if the market falls (often getting a better average entry). On the ask side similarly. This increases our passive presence but can be seen as a hybrid approach to execution – essentially having a preset plan to dynamically “buy the dip, sell the rip” within our inventory limits. We have to be careful not to place orders so deep that they represent too large risk if a crash occurs. Typically, 2-3 layers spaced by a few ticks or based on volatility intervals is reasonable. If the top order fills, we might replace it and also manage inventory as usual.

In summary, quoting logic produces a bid and ask price around the current market, adjusted for inventory and market conditions. Normally, we sit at those prices and earn spread as trades hit us. But the algorithm will dynamically widen/narrow or pull these quotes in response to order book changes, recent trades, and risk levels. We combine passive income from spreads with protective and occasional aggressive moves to avoid being overly exposed. Pseudocode for quote calculation (excluding event handling) might look like:

```python
# Given current market data and inventory, compute desired quotes:
mid_price = (best_bid + best_ask) / 2
ref_price = mid_price  # start from mid

# Inventory skew adjustment:
ref_price -= inv_position * inventory_bias_factor  # pushes ref_price down if long, up if short

# Funding bias (for perpetual):
if instrument == "perpetual":
    ref_price *= (1 - expected_funding_rate * time_until_funding)
    # if funding_rate positive, this slightly lowers ref_price to favor short; vice versa for negative

# Volatility-based spread:
spread = base_spread + vol_factor * short_term_vol  # e.g., base_spread in dollars plus factor * volatility

# Microstructure adjustments:
if order_book_imbalance > imbalance_threshold:
    # more bids than asks -> tighten bid side a bit, widen ask
    bid_adjust = - small_offset
    ask_adjust = + small_offset
elif order_book_imbalance < -imbalance_threshold:
    # more asks than bids -> opposite adjustment
    bid_adjust = + small_offset
    ask_adjust = - small_offset
else:
    bid_adjust = ask_adjust = 0.0

# Calculate final quote prices:
desired_bid = ref_price - spread/2  + bid_adjust
desired_ask = ref_price + spread/2  + ask_adjust

# Ensure not to cross the current best prices:
desired_bid = min(desired_bid, best_ask - tick_size)
desired_ask = max(desired_ask, best_bid + tick_size)
```

The above pseudocode illustrates how we derive desired_bid and desired_ask each cycle. Next, we handle how to place and manage these orders in the market.

## Order Placement and Cancellation Logic

Efficient order management is critical for a market maker. We need to frequently update orders to stay near the market and avoid stale quotes, but also avoid useless churn. Below is the logic for how and when we place or cancel/modify our orders:

### 1. Placement Frequency and Triggering

We run our quoting algorithm in an event-driven loop, supplemented by a periodic timer. Key triggers include:
-	Order Book Update: Whenever there’s a notable change in the best bid/ask (or depth) that moves the mid-price by more than a small threshold, we recalc our quotes. For example, if the mid moves by more than, say, 0.02% since we last quoted, that’s a trigger to adjust. We subscribe to Deribit’s real-time WebSocket feed for order book changes, so we can react quickly.
-	Trade Executions (Fill Events): If our bid or ask gets (partially) filled, that’s an immediate trigger to act (update inventory and adjust or pull the opposite order).
-	Periodic Timer: We also set a timer (e.g. every 0.5 seconds or 1 second) to refresh quotes just in case we missed micro updates or to implement small random shifts (to avoid being too predictable). This ensures even in static conditions we renew orders (which can prevent getting stuck at back of the queue for too long).

In practice, an event-driven approach (reacting to market data pushes) is most efficient. The periodic cycle mainly ensures we re-check things if no event has happened in a while (which is rare in BTC). Our VPS latency (~tens of milliseconds) means we are not the first to react in a 1ms move, but by triggering on significant changes rather than every tiny tick, we focus on meaningful updates.

### 2. Order Placement Details

When placing orders on Deribit, we use limit orders with post-only (to avoid taker fees or accidental market orders). We will specify the price calculated (rounded to the allowed tick size) and a size for each order. The order size might be fixed or dynamic: for example, we might quote with 10% of our inventory limit on each side, or a set amount like 100 USD worth of BTC (0.0033 BTC at 30k) to start. We might increase size if spread is wider or confidence higher, and decrease size in high volatility or if near risk limits. To keep things simple, assume a constant size per order initially (adjusted if needed by inventory – e.g., if we are very long, perhaps we put a larger ask to get flat faster, and a smaller bid).

After computing desired_bid and desired_ask, we compare them to our current working orders (if any):
-	If we have no active orders (e.g., just started or canceled due to an event), we simply place new orders at those prices.
-	If we have existing orders, we check if they need to be moved. Typically, we define a threshold for re-posting to avoid excessive cancel/replace: e.g., if the difference between current order price and desired price is more than half a tick or a certain percentage, we will cancel and repost. If it’s very minor, we might leave it to avoid losing queue priority for no reason. For example, if our current bid is 30000 and desired_bid is 29999.5, we might ignore that tiny difference if our order is still near top of book. But if the market moved and now best bid is 29980, our bid at 30000 is now too high (we’d be filled immediately at a bad price), so we definitely need to cancel or adjust down.

Posting Strategy: We generally aim to have our orders at or near the best bid/ask to maximize fill chances, but not always the very best price if conditions warrant caution. Sometimes, if our model says a certain price but another market maker is already there slightly better, we have to decide whether to match/become the new best or stay a tick behind. A common tactic is to join the best price if our edge allows it; if the spread is still above our minimum and we can sit at the best bid/ask, we do it. If the best price seems too aggressive (maybe a competitor quoting very tight), we may deliberately place our order a tick or two behind the best. This way, if an informed taker comes, they hit the best (our competitor) first, and we either get skipped (avoiding a toxic trade) or get filled slightly after at a better price if the move was real. Being just behind the front of the queue can sometimes protect us, at the cost of lower fill probability. We can configure this behavior: e.g., if volatility is high or our inventory is at risk, we hang back; if conditions are normal and we want fills, we jump to the front.

### 3. Cancellation and Refresh Conditions

We will cancel and re-place orders in a few scenarios:
-	Mid-Price Shift: If the market mid moved enough that our quotes are now off-center by more than a small band, we need to recentre. For example, if our quotes were at 29950 bid / 30050 ask (mid 30000) and now the market moved such that mid is 30100 (bid/ask maybe 30050/30150), our old quotes are way too low. The ask at 30050 would have been hit already (we’d have gotten filled) or if somehow not, it’s now far below market (we should cancel immediately to not give a free bargain to takers). So any substantial move in price requires cancelling outdated orders and recalculating new ones around the new mid. In practice, we continuously monitor the top of book; if best bid or ask moves away from our price by more than, say, a threshold (like 0.1%), we cancel and update.
-	Staleness Timer: Even if price hasn’t moved much, sitting with the same order for too long can be risky (market dynamics can change). We might implement a safety cancel after, say, 30 seconds or 1 minute of no fill just to refresh our quotes (possibly at the same price if still valid, but this resets queue priority – a downside – so this parameter needs tuning). Alternatively, instead of time-based, use a count of trades that have happened around our price. If many trades occurred and our order wasn’t filled, it might mean we were just outside the action; refreshing might not help unless we adjust price, so time-based might be simpler.
-	Quote Undercutting: If another maker places an order that undercuts ours (e.g., we were the best bid at 30000, and someone else comes in at 30001), then we’re no longer best. We must decide whether to cancel and improve our bid to regain priority, or stay put. This is a strategic decision: constantly matching every undercut can lead to “quote wars” and minimal profit spreads. Often, we set a limit on how tight we’re willing to go. If our model’s desired price is still 30000 and someone is at 30001, perhaps we let them have it for now and keep our bid, because improving would violate our pricing model (and maybe make the trade unprofitable after fees). However, if the spread is still large and we have room, we might move up to 30001 to remain competitive. The strategy could be: if new best price appears that is within a certain small delta of our own price, adjust ours to one tick beyond it (join the race); if it’s significantly away (someone is quoting extremely tight), then back off. We always ensure not to place orders that cross the mid or the other side’s best price – our quotes must remain a proper bid below best ask and ask above best bid.
-	Fill Events: When an order gets filled, an immediate cancellation of the opposite side quote is usually prudent. For example, if our ask is hit (we sold BTC), we should typically cancel our bid at once. This is because a taker just took our ask likely due to upward price pressure – the market might be about to trade higher, and our bid could fill at a now inferior price. By cancelling, we avoid being instantly long (buying on the bid) right after selling on the ask, which could end up a loser if price is moving up. Similarly, if our bid is hit (we buy), cancel the ask because price may be dropping. This practice is called “one-sided fill cancellation” – it protects from getting two fills in the same move (which would leave us flat but possibly with a loss on the second leg if the market moved too fast). Once we cancel, we typically enter a very short post-fill hold (maybe a second or two) to observe the price action, then re-enter with fresh quotes according to the new situation. If the fill was small or conditions still seem fine, we might reintroduce the other side sooner but usually at an updated price.
-	Partial Fills: If our order is partially filled, we have to update inventory and possibly adjust the order’s remaining size or price. Generally, if a partial fill occurs, it means our price was touched. Often we will cancel the remainder of that order and refresh it to maintain priority (the part left might now be behind in queue if multiple got filled at that level). However, if it’s a tiny fill and most of our order remains, we could leave it if it’s still at a good price. The safe approach: cancel the remainder and recalc both sides (with new inventory) before re-entering. This avoids scenarios where we treat a partial differently than full – consistency is easier.
-	Error/Disconnect Handling: Although not a primary design point, any robust strategy should handle cases like losing connection or an order reject. For example, if an order doesn’t successfully place or cancel, or we disconnect momentarily, on reconnect we should cancel all to avoid ghost orders and then resume. This is more implementation detail, but worth noting for completeness in code.

Order Throttling: We will implement minimal delays to avoid hitting API rate limits or creating excessive order churn. Deribit’s API allows a high rate of submissions, but we don’t want to constantly send hundreds of changes per second with 0.9 BTC capital – it’s unnecessary and can accrue cancel fees if any. A reasonable approach is to limit updates to, say, at most 5-10 per second. Our event logic naturally throttles since we react mainly to actual changes. We can also batch updates (update both quotes together rather than serially to avoid race conditions where one side is updated and not the other).

In pseudocode form, a simplified order management loop might look like:

```python
while True:
    # Receive market data or timer tick
    update = get_market_update()
    if update.type in ("orderbook_change", "timer"):
        desired_bid, desired_ask = compute_quotes()  # using logic from prior section
        # Compare to current orders
        if need_adjust(current_bid_order, desired_bid):
            cancel_order(current_bid_order)
            place_order("buy", desired_bid, size=order_size, post_only=True)
        if need_adjust(current_ask_order, desired_ask):
            cancel_order(current_ask_order)
            place_order("sell", desired_ask, size=order_size, post_only=True)
    if update.type == "fill":  # one of our orders got executed
        update_inventory(update)  # adjust position
        if update.side == "buy":   # our bid filled, we bought inventory
            cancel_order(current_ask_order)  # pull other side
        if update.side == "sell":  # our ask filled, we sold inventory
            cancel_order(current_bid_order)
        # possibly wait a short moment to reassess after fill
        desired_bid, desired_ask = compute_quotes()
        # place new orders based on updated inventory and market
        place_order("buy", desired_bid, size=order_size, post_only=True)
        place_order("sell", desired_ask, size=order_size, post_only=True)
```

This is a high-level sketch. In practice, we would incorporate more conditions as described (e.g., not immediately replacing both orders if we want to stagger re-entry, adjusting size, etc.). But it shows the flow: on data updates, adjust orders; on fills, handle inventory and cancel other side; then re-quote.

## Inventory and Risk Management

A core principle of market making is to remain market-neutral or at least keep inventory oscillating around zero. Large directional inventory is risky – the market could move against a held position and wipe out many spreads’ worth of profit. Our strategy implements strict inventory and risk management rules:

### 1. Position Limits

We set an inventory limit in BTC (or in USD notional) that we will not intentionally exceed. This limit is based on our capital (0.9 BTC) and risk tolerance. For example, we might limit to \(\pm 0.3\) BTC inventory as a hard cap (roughly 33% of capital exposure unhedged at any time). Or in notional terms, if using leverage, perhaps around $10,000–$20,000 worth of BTC. The idea is that a sudden 5% move against a 0.3 BTC position is a ~0.015 BTC loss (~1.7% of capital), which is significant but not devastating. If we allowed a full 0.9 BTC long or short, a 5% adverse move is 0.045 BTC (~5% of capital), and a 20% black swan move is 0.18 BTC (20% of capital) – still perhaps manageable but larger. We choose a limit that we are comfortable holding if the market becomes illiquid or if we decide to ride out a short-term adverse move.

In implementation, this means: if our inventory is at the limit on one side, we stop quoting that side entirely and only quote the other side to naturally get filled back towards neutral. For instance, if our long inventory hit the +0.3 BTC cap, we would cease placing bid orders (no more buying) and only place asks (to sell off inventory). This naturally will bring inventory down as those asks fill. We might even aggressively reduce by placing asks a bit lower than normal (inventory skewing) to get filled faster. Similarly, if at the short limit, we stop selling more and only bid to cover.

### 2. Inventory Target and Rebalancing

Our target inventory is zero (flat), meaning we are equally willing to be slightly long or short, but we don’t aim for a directional position. The inventory skewing of quotes (reservation price shift) is our primary mechanism to rebalance – it continuously nudges us back toward zero inventory by making one side more attractive to the market. Over time, if the market is mean-reverting, these small skew adjustments will tend to bring us back to neutral (we buy low, sell high by leaning against the price moves).

However, markets can trend for a while, and even a skewed quote might keep getting hit on one side. If we find that inventory isn’t reverting (e.g., we’ve been long for a while and the market keeps dropping, or vice versa), we have additional measures:
-	Position-based Hedge (intra-Deribit): One powerful approach, given we trade on Deribit exclusively, is to hedge using a different instrument. For example, if we are long 0.3 BTC on the perpetual and we’re near our risk limit, we could open a short position of equivalent size on a BTC futures contract (expiring future) to offset. This locks in a spread (basis) but neutralizes further directional risk – effectively we’ve converted our position into a basis trade. Our P/L from that point will be mostly the funding payments and basis change, rather than BTC price moves. We can continue market making around those positions, and later unwind the hedge when conditions are favorable. This is an advanced technique and requires monitoring of both instruments’ positions. The strategy would treat the combined inventory (perp + futures) in terms of net BTC exposure. If net exposure is near zero (long in one, short in another of same size), we’re effectively hedged. We then focus on reducing whichever side we want when possible. For instance, if we hedged a long perp with a short future, as soon as the market stabilizes or rises a bit, we might sell some perp or buy back the future to realize profit on that hedge.
-	Stop-Loss for Inventory: In extreme cases, if the market moves very far very fast (say a quick 10% move against our position) and our inventory is stuck, a prudent strategy might just stop out. That is, accept the loss and clear the position to avoid potential liquidation. For example, if we’re long and the market tanks rapidly, rather than doubling down or hoping for a bounce, our system could trigger a stop-loss: e.g., if unrealized loss exceeds a threshold (like 0.1 BTC, just as an example), then close the position at market. This is more of a catastrophic protection. Ideally, our position limits and hedging prevent reaching this point, but it’s good to define an absolute max pain threshold.
-	Sizing and Leverage: We use relatively small order sizes per quote relative to our capital, which naturally limits inventory accumulation speed. If each order is, say, 0.05 BTC and we have a 0.3 BTC limit, it would take 6 back-to-back fills in one direction to max us out. In a fast move, that could happen, but normally there are oscillations where we’d offload some before getting all six fills one way. By adjusting order size or temporarily pausing quoting after successive fills in one direction, we can further slow the accumulation. Additionally, we likely won’t use maximum leverage. Deribit allows up to 50x or more on futures, but we might use only 5x or 10x effectively, meaning our 0.9 BTC margin supports perhaps 4–9 BTC of positions comfortably. That means a 10% adverse move on full exposure would cost 0.4–0.9 BTC (which is 44–100% of capital, obviously too high). By keeping inventory small and often hedged or quickly flipped, we rarely approach a scenario of margin call. We always want enough free margin so that even if our inventory draws down, we don’t get liquidated. The strategy should monitor margin usage and keep it below, say, 50% at all times for safety.

### 3. Hedging Logic and Instrument Choice

As mentioned, on-exchange hedging can be done by using the perpetual and futures in tandem. For example: if we predominantly market make on the perpetual but accumulate a large long there, we could short an equivalent amount of the next quarterly future. This locks in the basis (the difference between perp and future price) as our profit or loss, and eliminates further BTC/USD exposure. We then effectively hold a calendar spread position. Because the question specifically says Deribit only, we won’t hedge on other exchanges or in spot. But Deribit does now offer some spot and linear futures; theoretically one could hedge with spot if available, but usually futures suffice.

The decision to hedge or not can depend on market conditions:
-	If funding is very high and we are long perp, hedging by shorting a future can be attractive: we’ll gain funding payments on the perp for as long as we hold the hedge, and the future we short has no funding to pay (just eventually converges). We must be aware that as expiry nears, that future short will converge to index, effectively yielding a profit if perp was trading rich initially (or we can unwind both at some point).
-	If we are short perp and funding is very high (meaning we pay funding), we might prefer to flatten that short either by buying perp or hedging via another future or reducing position quickly before funding hits.

The strategy’s risk manager component will thus periodically check:
-	Net exposure across instruments (if using both).
-	Upcoming funding times and rates.
-	Inventory vs limits.

If a hedge is placed, the system should mark that and manage it – e.g., not immediately try to “unwind” inventory via quoting because it’s already hedged. Instead, it might continue market making but on the hedged instrument preferentially, or work to reduce the hedge in a profitable way (like leg out when the basis moves in our favor).

For simplicity, if the initial strategy is too complex with hedging, one might first implement without cross-instrument hedges. In that case, risk management relies solely on reducing inventory via quoting and occasional taker trades to cut risk. That is acceptable if limits are small. We highlight hedging as an available tool for advanced risk management and to exploit the perp vs future basis when advantageous.

### 4. Order Size and Scaling

Inventory management also ties into how we scale order sizes. We might adopt a scaling strategy where, if inventory is small (near 0), we quote with our normal size. If inventory grows, we could decrease our quoting size on the side that would further increase it, and possibly increase size on the side that will reduce it. For example, if we are long 0.2 BTC (out of 0.3 limit), we might cut our bid order size in half (so buying less if price falls further), but double our ask order size (so if price rebounds, we sell more and reduce inventory faster). This dynamic sizing helps pull us back to neutral more aggressively when needed, and conservatively slows down accumulating more in the wrong direction. It’s like a proportional controller: the further from zero we are, the more we push toward zero.

### 5. Monitoring P&L and Risk Metrics

We will also constantly monitor unrealized P&L on our inventory and realized P&L from trades. If we notice that a series of trades has led to a drawdown beyond a certain daily limit (say we’re down 0.02 BTC on the day due to getting caught in a trend), the strategy could reduce activity (widen spreads further, or even stop trading for a cooling-off period). This is a precaution to avoid spiraling losses on a bad day. Many professional market makers have a “kill switch” that turns off trading if losses hit a threshold – this prevents a code bug or unexpected market scenario from draining all capital. We can implement a soft version: e.g., if loss > 0.05 BTC, then trade only minimal size or pause and alert the operator.

In summary, inventory and risk management ensures the strategy survives bad scenarios and remains in a position to profit from normal trading. By limiting position size, skewing quotes to mean-revert inventory, and hedging or cutting losses when needed, we aim to control risk tightly.

## Execution Tactics and Latency Considerations

Execution tactics refer to how we actually execute our strategy given practical constraints like latency and API mechanics. With a non-colocated VPS setup, our latency to Deribit might be on the order of tens of milliseconds. This is relatively fast for a human but slow in the realm of algorithmic trading (top firms colocated might have ~1ms latency or less). We must therefore design our execution to be robust against faster players and minimize the impact of our latency disadvantage.

### 1. Latency Optimization

Even without colocation, we can optimize by choosing a server geographically close to Deribit’s matching engine (Deribit’s servers are in London for their main engine as of last known, so an AWS London or similar could yield ~1-2 ms ping). Using Deribit’s WebSocket feed for market data gives us the fastest updates. We also use the HTTP or RPC API for order placement – Deribit’s APIs are quite performant but we should reuse persistent connections or their recommended interface to shave off ms.

We cannot outpace HFTs in reacting to cross-exchange arbitrage, but we can ensure our software is efficient (non-blocking, handling events concurrently). We also might implement small latency buffers: for instance, if we get a quote update signal and send a cancel, there’s a fraction of time our old order is still live. In that window, if a sudden trade hits it, we could get an unintended fill. To mitigate this, one could use Deribit’s edit order functionality (if available) to adjust price without leaving the book, or ensure to check for fills after cancel. Typically, we will do: cancel, wait for confirmation, then place new order – this prevents overlapping orders. The slight delay is acceptable.

Another tactic is using “protection” offsets: because we know we’re slower, we might quote a tiny bit further from the edge than we normally would, to reduce the chance that by the time our order reaches the order book, the market moved through it. For example, if we want to be at $30,000 bid and market is moving down, by the time our order posts, perhaps the best bid is $29,990. If we had placed at 30,000, we get filled instantly at a worse price. If we had been slightly behind (29,990), we might avoid that immediate adverse fill. In practice, we might incorporate a “latency padding” in fast markets: e.g., when volatility is high, shift quotes one extra tick away from mid than we otherwise would, to account for the latency. This is a form of slippage insurance.

### 2. Capturing Spread vs. Avoiding Adverse Selection

A delicate balance in execution is how aggressively to chase fills. As a market maker, we primarily want to earn the bid-ask spread. We do this by providing liquidity and letting others trade into our orders. However, as discussed, if those others have better information (e.g., they see a price move on another exchange and then take our quote), we end up with a position that immediately loses (this is adverse selection). Our execution tactics to address this include:
-	Widening During Volatility: As noted, we dynamically widen our spread when rapid moves are detected. This reduces the frequency of trades but each trade we do make is at a safer price. Essentially, when the market is stormy, we step back. This is observable on exchanges: spreads often widen in volatile moments. Our strategy follows that behavior to stay safe.
-	Staggered Quotes and Layering: Placing multiple small orders at different tiers can sometimes reduce adverse selection. If a sharp taker wants to buy 1 BTC and we had, say, 0.1 BTC on the best ask and another 0.1 BTC 0.5% higher, etc., they might only take the first 0.1 BTC and the rest of their order goes to other sellers or moves price. We got partially filled at a decent price and can adjust the rest. If instead we had a full 0.2 BTC at one price, we’d give them more volume at the worst price. So splitting orders into smaller chunks can limit how much gets filled at a single stale price. It also provides flexibility: we can cancel remaining layers if we see the first layer go and suspect momentum.
-	Conditional Orders or Kill-Switch: If Deribit offers stop-orders or similar, we likely won’t rely on those for market making (too slow), but we can implement our own checks. For instance, if we just sold on our ask and the market keeps surging up (meaning we likely sold low and now price is higher), we might decide not to re-enter a sell immediately. We could even have a rule: after a fill, wait \(X\) milliseconds and see if the price moved further in that direction by more than some threshold. If yes, perhaps hold off quoting further until things settle or adjust the base price significantly. This prevents us from being a punching bag in fast trending scenarios.
-	Queue Position and Refresh: Being near the front of the queue is good for fills but bad if those fills are toxic. One idea: if we suspect toxic flow (e.g., other exchanges moving), we might prefer to be slightly behind another order as mentioned. Let someone else be the first to get hit; we might get filled second, or have time to cancel. In practice, this is hard to time perfectly, but not always trying to be #1 in queue can avoid some toxic trades. On the other hand, if no one is in queue (empty order book momentarily), we do want to take that spot because there’s no buffer otherwise.
-	Use of Deribit Features: Deribit might have specific order types like reduce-only (to ensure an order only reduces net position and not increase it – useful if we only want to offload inventory at some price without accidentally adding) or good-till-cancel vs immediate-or-cancel. We will mostly use good-till-cancel limit orders for standing quotes, but for any aggressive action (like hedging or inventory dump) we’d use immediate-or-cancel or fill-or-kill to avoid resting accidentally. There’s also the concept of RFQ/block trading on Deribit for large sizes, but that’s outside our scope since we’re doing on-screen market making with small size.

### 3. Avoiding Self-Trades and Order Collisions

If we market make on both the perpetual and futures simultaneously, we need to ensure we don’t unintentionally trade against ourselves (e.g., our own bid on the future hits our own ask on the perpetual in some arbitrage scenario). This is generally not possible directly on one exchange’s matching engine (they typically won’t match your orders with each other), but across instruments, if we had separate sessions, it could conceivably happen through an intermediary price movement. We just note that the strategy should coordinate positions so that we’re not doubling risk. Usually, a single trading bot controlling both will naturally avoid that, but it should be monitored.

### 4. Example Scenario Execution

To tie together execution tactics, consider a concrete scenario:
-	The BTC perpetual is at $30,000 (30000/30005 spread). Our strategy sets a bid at 29990 and ask at 30010 (wider than market because volatility picked up). We’re slightly behind the best quotes which are 29995/30005 by other makers. A moment later, a large buy order comes in and eats the asks up to 30100. Our ask at 30010 was behind the best, so it might not have been filled at all if the best asks were sufficient to fill the buy. If it was filled, it’s at a relatively good price (30010, whereas price went to 30100). Right after seeing a sweep, our bot cancels any remaining orders (e.g., our bid) immediately. Now the best bid might be 30050 and best ask 30100 after the sweep. We update our inventory (if our ask filled, we are now short some amount). Given the bullish momentum, our risk management might either hedge that short via a short-term long on the futures or decide to quote only a bid now to cover it. We might place a bid at 30060 to try to buy back what we sold, aiming to capture that new wider spread (we sold at 30010, try to buy at 30060, that would net a $50 profit if successful, plus we earned maker rebate on the sale). If the momentum continues up, we might not get that buy, so after a bit, maybe we move it up or just take a small loss by lifting someone’s ask to cover if needed. This kind of reactive play shows how we combine passive fills with occasional follow-up aggression to manage inventory.

Throughout, the system logs and evaluates how often we’re getting filled and whether those fills are proving profitable or quickly turning into adverse moves. If the latter, it might indicate we need to widen more or adjust our signals – a continuous calibration process.

## Profitability Expectations and Considerations

A market making strategy typically yields many small profits (the earned spreads) and occasional losses (from adverse selection or inventory cuts). Here we outline what to expect in the context of Deribit BTC futures, including fees, typical returns, and the impact of market regimes:

### 1. Fee Impact and Trade Counts

On Deribit, assuming standard fees (0.025% per side for makers, and maybe 0.075% for takers), our goal is to do the vast majority of volume as maker. If executed correctly, >90% of our fills should be passive (earning rebates or at least low fees) and <10% aggressive (only when needed to hedge or stop-loss). This way, fees don’t eat the profits. For instance, if we trade $1,000,000 notional in a day (which is plausible with many small cycles), at maker 0.025% we’d pay $250 in fees, but if those were all maker with rebate, we’d earn $250 instead. That difference can be the difference between profit and loss. Therefore, we design the system to flag any unnecessary aggressive trades. We explicitly incorporate that in logic: e.g., always use post_only so an order won’t execute as taker even by accident (Deribit will reject if it would cross).

Each round-trip trade (a buy fill and a sell fill to flat the position) yields roughly “spread – fees” in profit. Suppose on average we capture a $20 spread on a $30k BTC (0.067%). If both legs are maker, fees might effectively be zero or even add a tiny rebate. If one leg ends up taker, we lose perhaps $15 on that leg, netting only $5 profit or even a slight loss. So the strategy’s profitability hinges on keeping that spread capture significantly above fees and minimizing taker incidents.

Expected Trade Frequency: With 0.9 BTC capital and moderate risk, we might be quoting maybe $5k-$10k size total (0.15–0.3 BTC) at any given time (split in orders). If the market is somewhat active, it wouldn’t be surprising to get perhaps 50-100 partial fills per hour in choppy conditions (just an estimate). That could be, say, 20-50 full round trips a day in normal markets. If each round yields 0.0005–0.001 BTC profit (which is $15–$30 at 30k, i.e. 0.05-0.1% of notional traded), then per day we might make on the order of 0.01-0.02 BTC ($300–$600) – this is a rough optimistic figure. That would be a 1-2% daily return on capital, which is quite high. In reality, competition and losses will likely bring it down. Even a few bad trades can wipe out dozens of tiny gains. So a more conservative expectation might be aiming for 0.002-0.005 BTC per day ($60-$150), which is about 0.3-0.7% daily on capital, still extremely high annualized (~hundreds of %). These numbers illustrate the potential but actual results will vary widely with market conditions.

### 2. Volatility Regimes

Low Volatility: In quiet, range-bound markets, our strategy should perform well. Spreads are tighter, so we make less per trade, but also adverse moves are smaller. We can tighten our quoting, get more frequent fills, and inventory mean reverts easily as price oscillates in a range. In such conditions, it’s common to see stable profits from market making as long as there’s some volume. The risk is that very low volatility also sometimes means very low volume – fewer trades happen, so we might not get filled often unless we aggressively price. But if we chase too tight, we risk being the only one quoting and then one large order could still hit us. Generally though, a low-vol steady market is a market maker’s comfort zone: we might adjust parameters to be more relaxed (smaller spreads, larger size) when we detect volatility (e.g. measured by average true range or high/low range) has been low.

High Volatility: In volatile markets (for example, during a big news event or a sudden Bitcoin price breakout), market making is challenging. Spreads on Deribit’s order book will widen as everyone pulls quotes (to avoid being run over). Our strategy will do the same – widening quotes and possibly reducing size. We may still get filled, but any fill has a higher chance of immediately going “out-of-the-money” as the price continues moving. This is where our defensive tactics (cancelling on momentum, hedging, etc.) are truly tested. It’s possible in a sharp move that we take a loss (sell low, then market shoots up, forcing us to buy back higher, or vice versa). Those losses can eat a lot of small spread profits. The key is that our risk management prevents catastrophic loss – e.g., by stopping trading if things are too wild or by quickly hedging out when needed. Interestingly, high volatility can also be very profitable if handled well: wider spreads mean each trade yields more profit, and if the market oscillates (high vol doesn’t always mean straight line trend; it could be mean-reverting with big swings), a market maker can earn much more per round trip. Some of the best days for market makers are volatile days where they manage to buy very low and sell very high repeatedly within a day. But also the worst days are volatile trending days where every fill is a loser because the trend persists. Our strategy attempts to shift from liquidity provider to quasi-momentum follower when a clear short-term trend is detected (e.g., stop offering into a spike). This way, during a one-directional violent move, we minimize fills (thus minimize losses), and when volatility is more two-sided, we capture the big spreads.

Regime Detection: We can incorporate simple metrics to detect regime: e.g., recent volatility level, and recent directional drift. If volatility beyond a threshold, switch to “defensive mode” (wider spreads, smaller size, maybe only one side quoting if directional). If volatility is low, use “normal mode” (tight spreads, symmetric quoting). This kind of adaptive behavior maximizes our profitability across regimes.

### 3. Profit Factors Specific to Deribit
-	Funding Gains/Losses: For perpetual swaps, funding will add or subtract to our profit. If we often end up net long during times of positive funding, we’ll be paying funding regularly (hurt P&L), whereas if we are net short in those times, we’ll be earning funding. A smart market maker might intentionally skew inventory strategy to earn funding. For example, if funding is consistently +0.01% every hour (annualized ~87% – a scenario of high long demand), being short on average yields that as extra income. We could slowly bias our inventory to short (e.g., allow a slightly larger short position than long, or keep quotes such that we tend to accumulate short when possible). However, we must be cautious: funding is usually high because price is rising (long-demand); being short against a strong uptrend can be deadly even if you earn funding. So this is only a minor consideration – we won’t compromise risk management just for funding. But in moderate conditions, it’s a nice boost. The strategy will likely often be near flat during funding, so it might not gain or lose much from it. We might explicitly ensure to go flat right before funding times to avoid any surprise large payment.
-	Maker Rebates: If Deribit offers rebates for maker volume (as was the case historically), our profit gets a small constant boost. For example, 0.025% rebate on each passive fill means even if we scratch a trade (buy and sell at same price), we’d net 0.05% of notional in rebates profit. That encourages high volume market making. We should confirm the current fee schedule; if rebates apply, we definitely want to maximize passive fills. If not (flat fee), our math just has to ensure spread > fees. This strategy can be tuned either way. Given our plan already emphasizes passive trading, we naturally benefit if any rebate exists.
-	Competition and Slippage: On Deribit, there are professional market-making firms (possibly with automated strategies similar to this). More competition means thinner spreads and harder to get filled (because someone is always a bit faster or more aggressive). This could reduce our strategy’s profitability, as we have to either accept a smaller edge or sit out when we can’t compete. One advantage of Deribit is that it’s not as crowded as, say, Binance futures with dozens of HFT firms – it’s liquid but a bit more specialized (especially with a lot of options volume). Our non-colo strategy might still have room to earn if we find niche opportunities (like slightly longer-term mean reversion rather than microsecond arbitrage). It’s worth noting that the real profits of market making often come from subtle advantages and occasionally from the information gleaned. We won’t rely on any exclusive info, but over time our fills themselves give us some signal (if we keep getting filled on one side, that says something about market bias that we can react to).

In summary, the strategy aims for consistent if modest profitability. We might expect a Sharpe ratio that is high (returns with low variance relative to many directional trades) but punctuated by occasional drawdowns. Success will depend on careful tuning and fast reactions to market changes.

## Perpetuals vs Expiring Futures: Trade-offs and Conditions

The strategy is designed to operate on both the BTC perpetual swap and the expiring BTC futures on Deribit. Both are similar in that they are linear derivatives on BTC price, but they have differences that affect our market making approach. Here we discuss when to use one, the other, or both, and what adjustments are needed:

### 1. Liquidity and Volume

BTC Perpetual is typically the most liquid instrument on any crypto futures exchange. It has the highest volume and tightest spreads most of the time, since it’s the product most traders use for short-term speculation. BTC Expiring Futures (e.g., quarterly contracts) may have slightly lower liquidity except near their expiry or during specific market conditions. On Deribit, the perp will attract more day traders and arbitrageurs, whereas the quarterly futures attract longer-term basis traders and institutional players.

For a small market maker with 0.9 BTC capital, concentrating on the perpetual might yield more consistent action: more frequent fills and easier exit since there’s always trade interest. Expiring futures might have moments of inactivity where our orders sit unwFilled longer (especially if we choose a far expiry).

However, quoting futures can still be profitable and at times more profitable if competition is lower there. The spread on a quarterly future might be a bit wider in absolute terms (maybe $10-$20) compared to perp (maybe $5-$10) during normal times, simply because fewer market makers bother with it. If we provide quotes there, we might capture those wider spreads occasionally. The trade-off is slower fill rate.

Approach: We can decide to either focus on one instrument at a time or run the strategy on both in parallel (with capital split or in cross margin if Deribit allows). If running both, we must monitor combined exposure, as discussed. A sensible approach could be:
-	During most periods, focus on the perpetual (primary instrument).
-	During specific scenarios (like very high funding rates or approaching quarterly expiry when basis is changing), shift more quoting to the futures, or even play them against each other (market make the spread between perp and future).

### 2. Funding vs Basis

The perpetual has funding payments typically every 8 hours (Deribit’s funding is often paid at 0:00, 8:00, 16:00 UTC etc). This creates a scenario where holding a position in the perp over those times has an additional cost or benefit. Expiring futures, by contrast, have no periodic funding; instead, they often trade at a basis (premium or discount) relative to the index reflecting the expected average funding or interest rates until expiry.

When funding is mild or near zero: The perp and futures behave very similarly. There’s no strong financial reason to prefer one over the other. Perp’s price will stick close to the index. A market maker can treat them interchangeably in terms of quoting logic.

When funding is significantly non-zero: This is when the difference matters. For example:
-	If the perp is trading above the index and has a high positive funding rate (longs paying shorts a lot), the expiring future might be trading at a relatively lower premium (or even at a discount if people expect the price to mean-revert). In such a case, being short the perp yields funding income but one must weather potentially a price drift. The future, on the other hand, might be a safer short (no funding to pay, just wait till expiry when it converges to index). However, shorting the future could miss out on funding gains.
-	For our strategy, if we see persistent high funding, it means the market is in imbalance (either strong uptrend or strong demand to be long). We can exploit this by preferring to quote more on the perp if we lean short or more on futures if leaning long. For instance, if funding is +0.1% per 8h (annualized ~110% – quite high), being short perp is attractive. Our market making might then bias to accumulate short positions on the perp (since we earn funding). But we must be careful: such high funding usually means price has been rising (lots of longs). We could hedge that risk by longing the future simultaneously, locking in that basis. In effect, that is a separate arbitrage trade (short perp, long future to collect funding). This might be beyond the pure scope of market making, but a market maker can integrate it: if we naturally end up short perp, we can hold that short a bit longer to get the funding, and perhaps not be too eager to cover unless market moves. Similarly, if funding is very negative (shorts pay longs), being long perp is attractive; we might allow ourselves to carry a long bias on perp and hedge via shorting spot or a future if needed (though spot not in Deribit, so likely use future).

Basis trading opportunities: As expiry nears, futures basis (price difference from index) shrinks. Sometimes the future may lag the perp or vice versa. A market maker could provide liquidity on both and essentially arbitrage the basis with limit orders: e.g., if the future is overpriced relative to perp, one could place aggressive quotes to sell the future and buy the perp. But since we are not using external signals, we’d detect that only via Deribit’s own prices (which we have if we subscribe to both order books). This edges into multi-instrument arbitrage more than simple market making, but it’s a value-added strategy: market making the spread between perp and future. Deribit even has a feature for trading spreads directly. For implementation, doing cross-instrument quoting is advanced but doable – you quote not just the outright prices, but also watch the price difference. For example, if perp minus future price is higher than theoretical (meaning future too cheap), you could place a bid on future and ask on perp simultaneously. This way if they fill, you’ve essentially bought cheap future and sold expensive perp – locking in a basis that will converge. This is profitable and hedged. However, the user’s question seems to focus on making markets on each instrument individually rather than such arbitrage, so we won’t delve deeply. We just mention the possibility as part of trade-offs: trading both can open this low-risk profit opportunity if done carefully (with minimal additional capital since one leg margin offsets the other somewhat).

Simpler perspective for different conditions:
-	In a rapidly trending market (say BTC is shooting up), the perp will likely trade at a premium and have high funding. A market maker might find the expiring futures safer to quote because they won’t have to pay funding if they get short, and the future’s price will eventually come down if the trend reverts. But in the immediate term, the trend can still hurt you. Generally, if trend is strong, whichever instrument, you reduce size or widen spread. Possibly avoid being on the wrong side of trend – e.g., in a big uptrend, don’t keep stacking asks in perp that keep getting filled; either step out or hedge with futures.
-	In a sideways or mean-reverting market, the perpetual’s continuous nature is fine. Futures have the complication of expiry but otherwise similar.
-	Near an expiry date, an expiring future can get more volatile (as traders roll positions to next quarter, or speculators might cause price to swing towards settlement). Liquidity can thin out on the expiring contract on last day. It might be wise to stop making markets on a contract on its expiry day (except possibly very early in the day) to avoid last-minute craziness and settlement issues. Instead, roll over to the next active contract. Perpetual of course has no expiry, but note that large players sometimes create volatility around funding times or round hours.

Capital Allocation: With 0.9 BTC, if we run both perp and a future, we can either split capital (like 0.5 BTC margin allocated to each via subaccounts perhaps) or use cross-margin on one account for both. Cross-margin would allow using the full capital for both combined, which is more capital-efficient but one must be careful that losses on one can eat margin for the other. If we implement cross-market hedging (like short one, long the other), cross-margin is ideal since the positions offset in risk and margin requirements. If we treat them separate, we risk one side liquidating while the other is a winner we can’t realize. So likely, one combined account is better, but the logic must handle both together (complex but doable).

Given the technical level, the strategy could be initially implemented on the perpetual alone (where most action is) and then extended to futures once stable. The trade-offs above guide when to favor one:
-	High funding -> include futures to hedge or to avoid paying funding.
-	Approaching expiry -> possibly capitalize on basis or wind down quoting that contract to avoid turbulence.
-	High volatility -> stick to most liquid (perp) for quicker exit (futures might be less liquid in extreme moves).
-	Normal times -> perp for frequent small gains, maybe occasionally switch to futures if perp gets crowded or if futures offer better spread.

Ultimately, a combined approach can yield slightly more profit (via basis arbitrage and funding exploitation) but requires more complexity. The user, being technically experienced, could handle this by modularizing the strategy per instrument and adding a layer for cross-instrument logic.

## Implementation Outline (Pseudocode)

Finally, we summarize the strategy in a structured outline suitable for implementation. The pseudocode below sketches the major components and their interactions in a Pythonic style:

```python
# Pseudocode for Deribit BTC Market Making Strategy

initialize:
    capital = 0.9  # BTC
    instrument1 = "BTC-PERPETUAL"    # perpetual swap
    instrument2 = "BTC-DEC2025"     # example future
    inventory = {instrument1: 0, instrument2: 0}  # net positions
    inventory_limit = 0.3  # BTC, max exposure each instrument (for simplicity)
    gamma = some_value  # risk aversion for inventory skew
    base_spread = some_value  # e.g., 0.05% of price
    vol_factor = some_value   # factor to multiply volatility for spread
    order_size = 0.02 BTC (for example)
    last_quote_time = 0
    state = "normal"  # could also have "volatile" etc.

connect_to_deribit()
subscribe_order_book(instrument1), subscribe_order_book(instrument2)
subscribe_trades(instrument1), subscribe_trades(instrument2)

function compute_quotes(instrument):
    best_bid, best_ask = get_best_bid_ask(instrument)
    mid = (best_bid + best_ask) / 2
    # basic spread calc
    vol = calculate_short_term_volatility(instrument)  # e.g., std dev of last N mid price changes
    spread = base_spread * mid + vol_factor * vol * mid
    # ensure minimum tick multiple
    spread = max(spread, 2 * tick_size(instrument))
    # inventory skew
    inv = total_inventory_exposure(instrument)  # if using combined net, else inventory[instrument]
    ref_price = mid - gamma * inv * (vol**2 if vol else 1)  # if no vol data, treat vol^2 as 1 minimal
    # funding skew for perp
    if instrument == instrument1:  # perpetual
        funding_rate = get_next_funding_rate()  # e.g., 0.01%
        time_to_funding = get_time_to_next_funding()  # in hours
        # Adjust ref price slightly by expected funding impact:
        ref_price *= (1 - funding_rate * time_to_funding/8)
        # (if funding positive, reduce ref_price slightly to favor short; negative does opposite)
    # order book imbalance
    imbalance = (sum_bid_sizes(levels=5) - sum_ask_sizes(levels=5))
                / (sum_bid_sizes(5)+sum_ask_sizes(5))
    bid_adj = ask_adj = 0
    if imbalance > 0.3:  # e.g., significantly more bids
        bid_adj += tick_size(instrument)  # move bid 1 tick up
        ask_adj += tick_size(instrument)  # move ask 1 tick up (less aggressive ask)
    elif imbalance < -0.3:
        bid_adj -= tick_size(instrument)  # move bid 1 tick down (less aggressive bid)
        ask_adj -= tick_size(instrument)  # move ask 1 tick down (more aggressive ask to sell into heavy asks)
    # final prices
    desired_bid = ref_price - spread/2 + bid_adj
    desired_ask = ref_price + spread/2 + ask_adj
    # round to nearest valid tick
    desired_bid = floor_to_tick(desired_bid, instrument)
    desired_ask = ceil_to_tick(desired_ask, instrument)
    # Ensure they are not inverted
    if desired_bid >= desired_ask:
        desired_bid = ref_price - tick_size(instrument)
        desired_ask = ref_price + tick_size(instrument)
    return desired_bid, desired_ask

function update_quotes(instrument):
    desired_bid, desired_ask = compute_quotes(instrument)
    # decide on quoting based on inventory limits
    inv = inventory[instrument]
    if inv >= inventory_limit:
        # Too long -> don't place bid (no more buying)
        desired_bid = None
    if inv <= -inventory_limit:
        # Too short -> don't place ask (no more selling)
        desired_ask = None
    # Check and update existing orders:
    current_bid = current_order[instrument]["bid"]
    current_ask = current_order[instrument]["ask"]
    # Bid side
    if desired_bid is None:
        if current_bid exists:
            cancel_order(current_bid)
            current_order[instrument]["bid"] = None
    else:
        if current_bid is None:
            place_order(instrument, "buy", price=desired_bid, size=order_size, post_only=True)
            current_order[instrument]["bid"] = desired_bid
        elif abs(current_bid.price - desired_bid) > tick_size(instrument)/2:
            # Move order if price difference is significant
            cancel_order(current_bid)
            place_order(instrument, "buy", price=desired_bid, size=order_size, post_only=True)
            current_order[instrument]["bid"] = desired_bid
    # Ask side
    if desired_ask is None:
        if current_ask exists:
            cancel_order(current_ask)
            current_order[instrument]["ask"] = None
    else:
        if current_ask is None:
            place_order(instrument, "sell", price=desired_ask, size=order_size, post_only=True)
            current_order[instrument]["ask"] = desired_ask
        elif abs(current_ask.price - desired_ask) > tick_size(instrument)/2:
            cancel_order(current_ask)
            place_order(instrument, "sell", price=desired_ask, size=order_size, post_only=True)
            current_order[instrument]["ask"] = desired_ask

# Main event loop
on market_data_update(data):
    # throttle updates to, say, max 5 per second
    now = current_time()
    if now - last_quote_time < 0.2s:
        # skip too frequent updates
        return
    last_quote_time = now
    # if the update is significant (price move or depth change), recalc quotes:
    update_quotes(instrument1)
    update_quotes(instrument2)

on order_fill(event):
    instrument = event.instrument
    side = event.side  # "buy" or "sell" that was filled (from our perspective)
    qty = event.quantity
    # update inventory
    if side == "buy":
        inventory[instrument] += qty  # we bought base
    else:
        inventory[instrument] -= qty  # we sold base
    # If one side filled, cancel the other side quote for that instrument (to avoid double fill)
    if side == "buy" and current_order[instrument]["ask"]:
        cancel_order(current_order[instrument]["ask"])
        current_order[instrument]["ask"] = None
    if side == "sell" and current_order[instrument]["bid"]:
        cancel_order(current_order[instrument]["bid"])
        current_order[instrument]["bid"] = None
    # After a fill, decide if we need to hedge inventory across instruments:
    net_inv = total_net_inventory()  # if we consider hedging between perp & future
    if abs(net_inv) > hedge_trigger:
        hedge_instrument = (instrument1 if instrument == instrument2 else instrument2)
        # Place a hedge order on the other instrument to reduce net exposure
        hedge_side = "buy" if net_inv < 0 else "sell"
        hedge_qty = min(abs(net_inv), some_safe_amount)
        place_order(hedge_instrument, hedge_side, price=best_price(hedge_instrument, hedge_side),
                    size=hedge_qty, post_only=False)  # possibly taker to execute immediately
        # Update inventory for hedge_instrument accordingly
        inventory[hedge_instrument] += hedge_qty * (1 if hedge_side=="buy" else -1)
    # Recompute and place new quotes (maybe after slight delay to avoid race condition)
    schedule_task(delay=0.1s, func=update_quotes, args=(instrument,))
```

Note: Proper implementation must also include robust error handling, logging, and perhaps a sandbox/testing phase on testnet or with small size. Risk parameters (`gamma, base_spread, vol_factor, inventory_limit`) should be adjusted based on backtesting or paper trading to achieve a good balance of fill rate vs. safety.
