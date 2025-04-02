Example RPC to list subscriptions.

Interesting fields:

- `recurrence_id`
- `renew_state`
- `state`: Can be at least: `draft`, `sale`, and `cancel`
- `partner_id`: Customer id and name. Likely, we want a fixed link between a customer id and an "Organization" in our
  MTA
- `access_url` + `access_token`: Allows customer to manage their subscription
- `order_line`: Required for the next request to figure out the product
- `invoice_ids`: To display invoices later

`POST https://staging-remails.odoo.com/jsonrpc`

```json
{
  "jsonrpc": "2.0",
  "method": "call",
  "params": {
    "service": "object",
    "method": "execute_kw",
    "args": [
      "remails-16-0-sandbox-19290361",
      "<User id as int>",
      "<API KEY>",
      "sale.order",
      "search_read",
      [
        [
          [
            "is_subscription",
            "=",
            "true"
          ]
        ]
      ],
      {
        "fields": [
          "recurrence_id",
          "renew_state",
          "state",
          "partner_id",
          "access_url",
          "access_token",
          "order_line",
          "invoice_ids"
        ]
      }
    ]
  },
  "id": 11
}
```

```json
{
    "jsonrpc": "2.0",
    "method": "call",
    "params": {
        "service": "object",
        "method": "execute_kw",
        "args": [
            "remails-16-0-sandbox-19290361",
            "<User id as int>",
            "<API KEY>",
            "sale.order.line",
            "read",
            ["<order_line from previous query>"],
            {
                
            }
        ]
    },
    "id": 11
}
```