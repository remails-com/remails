INSERT INTO organizations (id, name, total_message_quota, used_message_quota, quota_reset, remaining_rate_limit,
                           rate_limit_reset, current_subscription)
VALUES ('44729d9f-a7dc-4226-b412-36a7537f5176',
        'test org 1',
        800, 0, now() + INTERVAL '1 month',
        500000, now() + '1 day',
        '{
          "title": "Mock testing subscription",
          "status": "active",
          "product": "RMLS-FREE",
          "end_date": null,
          "start_date": "2025-11-23",
          "description": "This is a mock subscription for testing only\nIt uses the same quota as the Remails Free subscription",
          "subscription_id": "mock_subscription_id",
          "sales_invoices_url": "https://tweedegolf.com/",
          "recurring_sales_invoice_id": "mock_invoice_id"
        }'); -- practically unlimited rate limit for testing quota

INSERT INTO organizations (id, name, total_message_quota, used_message_quota, quota_reset, remaining_rate_limit,
                           rate_limit_reset)
VALUES ('5d55aec5-136a-407c-952f-5348d4398204',
        'test org 2',
        500, 0, now() + INTERVAL '1 month',
        0, now() + '-1 day'), -- rate limit that should be reset
       ('533d9a19-16e8-4a1b-a824-ff50af8b428c',
        'quota reset test org 1',
        500, 0, now() - INTERVAL '1 minute',
        500000, now() + '1 day'),
       ('ee14cdb8-f62e-42ac-a0cd-294d708be994',
        'quota reset test org 2',
        500, 0,
        '2025-06-25 23:59:59.000000 +00:00'::timestamp with time zone - INTERVAL '100 months',
        500000, now() + '1 day'),
       ('7b2d91d0-f9d9-4ddd-88ac-6853f736501c',
        'quota reset test org 3',
        333, 0, now() + INTERVAL '1 minute',
        500000, now() + '1 day'),
       ('0f83bfee-e7b6-4670-83ec-192afec2b137',
        'quota reset test org 4',
        333, 0,
        '2025-01-31 23:59:59.000000 +00:00',
        500000, now() + '1 day');

INSERT INTO organizations (id, name, moneybird_contact_id, current_subscription, quota_reset)
VALUES ('ad76a517-3ff2-4d84-8299-742847782d4d',
        'webhook test',
        'webhook_test_org',
        '{
          "status": "none"
        }',
        now()),
       ('e11df9da-56f5-433c-9d3a-dd338f262c66',
        'First subscription become admin with two members',
        'first_subscription_test_org',
        '{
          "status": "none"
        }',
        now());
