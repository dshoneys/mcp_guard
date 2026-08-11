# REQ-VAULT-UI — mapping

| Logical | Preview | Native IPC / bridge |
|---------|---------|---------------------|
| View | `#view-vault` (secondary) | same HTML; home `#btn-open-vault` |
| Back | `#btn-back-home` | client-side → `#view-home` |
| Hint | `#vault-hint` | static / i18n |
| List | `#vault-list` / `#vault-empty` | `mcpGuardVaultApply({secrets})` |
| Save | `#vault-form` submit | `{"action":"vault-put","name","value"}` |
| Delete | row button | `{"action":"vault-delete","name"}` |
| Feedback | `#vault-note` + OS toast | never echo value |
| Clear plaintext | `mcpGuardVaultClearForm` | JS clears inputs after submit |
