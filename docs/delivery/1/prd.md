# PBI-1: Deribit Delta-Neutral Market Maker

## Overview
Implement the research strategy in a Rust codebase.

## Problem Statement
Automate passive and hybrid quoting on Deribit while managing risk.

## User Stories
- As a trader I want a bot that quotes both sides so that I can earn the spread.

## Technical Approach
Use WebSocket feeds for real-time data and HTTP/RPC for orders. Follow pseudocode in RESEARCH.md.

## UX/UI Considerations
CLI based configuration and logging.

## Acceptance Criteria
- Bot places quotes and manages inventory within limits.
- Uses Deribit-only data.
- Includes hedging logic and kill switch.

## Dependencies
None

## Open Questions
- Which Rust crates to use for WebSocket and HTTP?

## Related Tasks
[@View in Backlog](../backlog.md)
