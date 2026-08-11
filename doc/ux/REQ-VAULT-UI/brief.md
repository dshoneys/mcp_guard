# REQ-VAULT-UI — brief

## Goal

Manage vault secrets from the main dashboard: add / list / delete. Operators never need a terminal to store keys that agents will consume via NoContext MCP.

## Jobs

1. Add a named secret (value entered once; not shown again)
2. See names only in a list
3. Delete a secret with confirm feedback
4. Understand that agents get **refs / scrubbed runs**, not plaintext in chat

## Feedback

- Save success → toast + list refresh (never echo value)
- Delete success → toast + list refresh
- Errors → toast

## Non-goals

- Reveal/view plaintext in UI
- Cloud sync
- Sharing secrets across machines
