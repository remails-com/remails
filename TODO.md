## TODO

### General

- Configuration based on file, env. etc. (1d)

1d

### SMTP server

x Move split connection module to session module (1d) (done)
- Implement all relavent SMTP commands (3d)
- LMTP (1d)
- Session timeout (1d)
- Other authentication methods? (1d)
- Accept STARTLS? (1d)
- Log sender IP (-)

8d

### Message handler

- Check limits (account etc.) (2d)
- Handle unparseable messages (1d)
- Spam filter -> SpamAssassin? (2d)

5d

### HTTP API

- Add company / customer entity / crud (1d)
- Add domain entity / crud (2d)
    - Domain verification (SPF/DKIM) (2d)
- Subscription endpoints / Odoo integration (3d)
- Webhook endpoints / caller (2d)
- Rename users to SMTP credentials (-)
- Add oauth / mail message auth flow (3d)
- Message content endpoint (-)
- User impersonation / user magament (1d)

14d

### Sending / forwarding

- See https://github.com/mikedilger/mailstrom (unmaintained but relevant)
- Use lettre as transport (2d)
- Add relay headers (1d)
- Resolve MX records (1d)
- Store delivery status (1d)
- Handle deferred status (-)
- DKIM signing (1d)
- Sender IP management (1d)

7d

### Frontend

- Use Mantine / vite
- User / admin interfaces (7d)

7d

Som: 40d
Onvoorzien: 7d
Testing / hosting integratie: 10d

57 dagen = 456 uur